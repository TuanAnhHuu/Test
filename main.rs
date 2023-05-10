use async_std::task;
use async_std::channel;
use rppal::gpio::{Gpio, Level};
use std::time::Duration;
use tokio::select;
use reqwest;
use serde::{Deserialize, Serialize};


const DELAY_SHORT: u64 = 100;
const DELAY_LONG: u64 = 1000;

async fn blink_led(rx: channel::Receiver<bool>) {
    
    let gpio = Gpio::new().unwrap();
    let mut led_pin = gpio.get(17).unwrap().into_output();
 
    let mut led_state: bool = false;
    let mut delay: u64 = DELAY_LONG;

    loop {
        select! {
            msg = rx.recv() => {
                if let Ok(button_state) = msg {
                    led_state = button_state;
                    delay = if led_state { DELAY_SHORT } else { DELAY_LONG };
                    println!("Task blink_led : delay {}",delay);
                }
            }
            _ = async_std::task::sleep(Duration::from_millis(delay)) => {
                led_state = !led_state;
                println!("Task blink_led: Led_state = {}",led_state );
                if led_state == false {
                    led_pin.set_low();
                } else if led_state == true {
                    led_pin.set_high();
                }
        
            }
        }
    }
}

async fn check_button(tx: channel::Sender<bool>, tx1: channel::Sender<bool>) {
  
    let gpio = Gpio::new().unwrap();
    let button_pin = gpio.get(18).unwrap().into_input_pulldown();
    let mut old_button_state = Level::Low;
    loop {
        let button_state = button_pin.read();
        println!("Task check_button: is runing");

        if button_state == Level::High && old_button_state == Level::Low {
            println!("Task check_button:Button_state = 1");
            tx.send(true).await.unwrap();
            tx1.send(true).await.unwrap();
        }
        task::sleep(std::time::Duration::from_millis(100)).await;
        old_button_state = button_state;
    }   
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    code: u32,
    data: AccessTokenData,
    metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenData {
    accessToken: String,
    refreshToken: String,
    role: String,
    consumerId: Option<u32>,
    producerId: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Metadata {
    appName: String,
    version: String,
    timestamp: String,
}

async fn send_post_request(_rx: channel::Receiver<bool>) -> Result<(), Box <dyn std::error::Error + Send + Sync>> {

    let mut state_login = false; 

    while let Ok(value) = _rx.recv().await {
        println!("Task send_post_request: value = {}", value);
        state_login = value;

        if state_login == true {

            let client = reqwest::blocking::Client::new();
            let url = "https://auth-platform.apis-staging.devr.com/api/v1/auth/login";
            let json_data = r#"
                {
                    "username": "user1@devr.com",
                    "password": "123456",
                    "excelLicenseId": "000-000-000-001",
                    "role": "PRODUCER"
                }
            "#;

            let response = client
                .post(url)
                .header("accept", "*/*")
                .header("Content-Type", "application/json")
                .body(json_data)
                .send()?;
            
            if response.status().is_success() {
                println!("Success:{}", response.status());
                
                let body = response.text()?;

                let response_json: AccessTokenResponse = serde_json::from_str(&body)?;
                
                // Access the values from the response
                let access_token = response_json.data.accessToken;
                let refresh_token = response_json.data.refreshToken;
                let role = response_json.data.role;
                let consumer_id = response_json.data.consumerId;
                let producer_id = response_json.data.producerId;
                let app_name = response_json.metadata.appName;
                let version = response_json.metadata.version;
                let timestamp = response_json.metadata.timestamp;

                // Print the accessed values
                println!("Access Token: {}", access_token);
                println!("Refresh Token: {}", refresh_token);
                println!("Role: {}", role);
                println!("Consumer ID: {:?}", consumer_id);
                println!("Producer ID: {}", producer_id);
                println!("App Name: {}", app_name);
                println!("Version: {}", version);
                println!("Timestamp: {}", timestamp);
            } else {
                println!("Error: {}", response.status());
            }

            state_login = false;
        } 
    }
    
    Ok(())
}

#[tokio::main]
async fn main() {

    let (tx, rx) = channel::unbounded();
    let (tx1, rx1) = channel::unbounded();

    
    let check_button_handle = task::spawn(check_button(tx.clone(),tx1.clone()));
    let blink_led_handle    = task::spawn(blink_led(rx1.clone()));
    let post_request_handle = task::spawn(send_post_request(rx.clone()));

    check_button_handle.await;
    blink_led_handle.await;
    post_request_handle.await.unwrap();
}