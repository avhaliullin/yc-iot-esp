use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use embedded_svc::{
    mqtt::client::QoS,
    wifi::{ClientConfiguration, Configuration, Wifi},
};
use esp_idf_hal::{delay, peripherals::Peripherals};
use esp_idf_svc::tls::X509;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    log::EspLogger,
    mqtt::client::{EspMqttClient, EspMqttMessage, MqttClientConfiguration, MqttProtocolVersion},
    nvs::EspDefaultNvsPartition,
    wifi::EspWifi,
};
use std::{mem, result::Result, slice};

fn convert_certificate(mut certificate_bytes: Vec<u8>) -> X509<'static> {
    // append NUL
    certificate_bytes.push(0);

    // convert the certificate
    let certificate_slice: &[u8] = unsafe {
        let ptr: *const u8 = certificate_bytes.as_ptr();
        let len: usize = certificate_bytes.len();
        mem::forget(certificate_bytes);

        slice::from_raw_parts(ptr, len)
    };

    // return the certificate file in the correct format
    X509::pem_until_nul(certificate_slice)
}

fn run_mqtt() {
    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs)).unwrap();

    wifi_driver
        .set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: "???".into(),
            password: "???".into(),
            ..Default::default()
        }))
        .unwrap();

    wifi_driver.start().unwrap();
    wifi_driver.connect().unwrap();
    while !wifi_driver.is_connected().unwrap() {
        let config = wifi_driver.get_configuration().unwrap();
        println!("Waiting for station {:?}", config);
    }
    println!("Should be connected now");
    delay::FreeRtos::delay_ms(1000);

    let client_cert_data: Vec<u8> = include_bytes!("certs/client/cert.pem").to_vec();
    let client_key_data: Vec<u8> = include_bytes!("certs/client/key.pem").to_vec();

    let client_cert = convert_certificate(client_cert_data);
    let client_key = convert_certificate(client_key_data);

    let mut mqtt_client = EspMqttClient::new(
        "mqtts://mqtt.cloud.yandex.net:8883",
        &MqttClientConfiguration {
            protocol_version: Some(MqttProtocolVersion::V3_1_1),
            client_id: Some("esp32"),
            disable_clean_session: false,
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            client_certificate: Some(client_cert),
            private_key: Some(client_key),
            ..MqttClientConfiguration::default()
        },
        move |message_event| {},
    )
    .unwrap();
    mqtt_client.publish(
        "$me/device/events",
        QoS::AtLeastOnce,
        false,
        "Hello from rust?".as_bytes(),
    );
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    run_mqtt();
    println!("Hello, world!");
}
