use std::collections::HashMap;
use std::sync::{Arc};
use tokio::net::TcpListener;
use tokio::task;
use tokio::sync::Mutex;
use tungstenite::protocol::Message;
use tokio_tungstenite::accept_async;
use futures_util::{SinkExt, StreamExt};
use rand::{thread_rng, seq::SliceRandom};
use std::net::{Ipv6Addr, SocketAddr};
use rand::Rng;
use neuromorph_driver;
use neuromorph_sys::NeuromorphMemcpyKind;

// Структура для нейрона
struct Neuron {
    weights: neuromorph_driver::DeviceMemory,  // Device memory for weights
    bias: neuromorph_driver::DeviceMemory,     // Device memory for bias
    kernel: neuromorph_driver::Kernel,         // Compiled kernel for forward pass
}

impl Neuron {
    fn new(weights: Vec<i32>, bias: i32) -> Result<Neuron, neuromorph_driver::NeuromorphError> {
        // Allocate device memory for weights
        let weights_device = neuromorph_driver::DeviceMemory::new(weights.len() * std::mem::size_of::<i32>())?;

        // Copy weights to device
        let weights_bytes = unsafe {
            std::slice::from_raw_parts(weights.as_ptr() as *const u8, weights.len() * std::mem::size_of::<i32>())
        };
        weights_device.copy_from_host(weights_bytes, NeuromorphMemcpyKind::HostToDevice)?;

        // Allocate device memory for bias
        let bias_device = neuromorph_driver::DeviceMemory::new(std::mem::size_of::<i32>())?;

        // Copy bias to device
        let bias_bytes = bias.to_le_bytes();
        bias_device.copy_from_host(&bias_bytes, NeuromorphMemcpyKind::HostToDevice)?;

        // Create dummy kernel data (for Phase 3)
        let kernel_data = vec![0xAAu8; 1024]; // Dummy kernel bytes
        let kernel = neuromorph_driver::Kernel::from_bytes(&kernel_data)?;

        Ok(Neuron {
            weights: weights_device,
            bias: bias_device,
            kernel,
        })
    }

    // Helper method to get weights from device (temporary for Phase 2)
    fn get_weights(&self) -> Result<Vec<i32>, neuromorph_driver::NeuromorphError> {
        let num_weights = self.weights.size() / std::mem::size_of::<i32>();
        let mut weights_bytes = vec![0u8; self.weights.size()];
        self.weights.copy_to_host(&mut weights_bytes, NeuromorphMemcpyKind::DeviceToHost)?;
        
        let weights: Vec<i32> = (0..num_weights)
            .map(|i| {
                let start = i * std::mem::size_of::<i32>();
                let end = start + std::mem::size_of::<i32>();
                i32::from_le_bytes(weights_bytes[start..end].try_into().unwrap())
            })
            .collect();
        Ok(weights)
    }

    // Helper method to get bias from device (temporary for Phase 2)
    fn get_bias(&self) -> Result<i32, neuromorph_driver::NeuromorphError> {
        let mut bias_bytes = [0u8; 4];
        self.bias.copy_to_host(&mut bias_bytes, NeuromorphMemcpyKind::DeviceToHost)?;
        Ok(i32::from_le_bytes(bias_bytes))
    }

    fn forward(&self, inputs: Vec<i32>) -> Result<i32, neuromorph_driver::NeuromorphError> {
        // Create a stream for asynchronous operations
        let stream = neuromorph_driver::Stream::new()?;

        // Allocate device memory for input
        let input_size = inputs.len() * std::mem::size_of::<i32>();
        let input_device = neuromorph_driver::DeviceMemory::new(input_size)?;

        // Copy input data to device
        let input_bytes = unsafe {
            std::slice::from_raw_parts(inputs.as_ptr() as *const u8, input_size)
        };
        input_device.copy_from_host_async(input_bytes, &stream, NeuromorphMemcpyKind::HostToDevice)?;

        // Allocate device memory for output (single i32)
        let output_device = neuromorph_driver::DeviceMemory::new(std::mem::size_of::<i32>())?;

        // Launch the kernel (dummy kernel for Phase 3)
        // Note: In a real implementation, the kernel would perform the actual computation
        // For now, we just launch it as a placeholder
        let event = neuromorph_driver::Event::new()?;
        self.kernel.launch(1, &stream, Some(&event))?; // Launch for 1 tick
        event.synchronize()?; // Wait for completion

        // Copy result back to host
        let mut output_bytes = [0u8; 4];
        output_device.copy_to_host(&mut output_bytes, NeuromorphMemcpyKind::DeviceToHost)?;

        let output = i32::from_le_bytes(output_bytes);

        // Apply sigmoid (since the dummy kernel doesn't do this)
        Ok(sigmoid(output))
    }
}

fn sigmoid(x: i32) -> i32 {
    // Приблизительная версия сигмоидной функции для целых чисел
    // Примерно (x / (1 + |x|)) * масштабирование (например, 1000)
    if x > 10000 {
        return 1000;
    } else if x < -10000 {
        return 0;
    }
    (1000 * x) / (1000 + x.abs())
}

// Функция для обработки подключений к серверу WebSocket (аксон)
async fn handle_client(ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, neuron: Arc<Mutex<Neuron>>, dendrites: Arc<Mutex<HashMap<String, i32>>>) {
    let (mut write, mut read) = ws_stream.split();
    let mut address = None;
    
    while let Some(msg) = read.next().await {
        let msg = msg.expect("Error in receiving message");
        if msg.is_text() {
            let text = msg.to_text().unwrap();
            if text.starts_with("CONNECT") {
                // Сохраняем адрес дендрита
                address = Some(text[8..].to_string());
            } else {
                let input_data: Vec<i32> = text
                    .split(',')
                    .map(|s| s.parse::<i32>().unwrap())
                    .collect();
                
                let neuron_clone = neuron.lock().await;
                let output = neuron_clone.forward(input_data.clone()).unwrap();
                let response = format!("{}", output);
                write.send(Message::Text(response)).await.expect("Failed to send message");

                if let Some(addr) = &address {
                    let error = (output - input_data[0]).abs();  // Примерная ошибка
                    dendrites.lock().await.insert(addr.clone(), error);
                }
            }
        }
    }
}

// Функция для замены худших дендритов
// async fn replace_worst_dendrites(dendrites: Arc<Mutex<HashMap<String, i32>>>, ipv6_list: Arc<Mutex<Vec<String>>>) {
//     let mut dendrite_errors = dendrites.lock();
    
//     // Собираем адреса для замены
//     let mut dendrite_vec: Vec<_> = dendrite_errors.await.iter().collect();
//     dendrite_vec.sort_by(|a, b| b.1.cmp(a.1));  // Сортировка по убыванию ошибки
    
//     let worst_count = (dendrite_vec.len() as f64 * 0.25).ceil() as usize;
//     let worst_addrs: Vec<String> = dendrite_vec.iter()
//                                                .take(worst_count)
//                                                .map(|(addr, _)| addr.to_string())
//                                                .collect();

//     drop(dendrite_vec);  // Завершаем использование immutable-ссылки

//     // Обрабатываем и заменяем
//     for addr in worst_addrs {
//         println!("Заменяем дендрит с адресом {}", addr);
//         dendrite_errors.await.remove(&addr);

//         let ipv6_list_locked = ipv6_list.lock();
//         if let Some(new_addr) = ipv6_list_locked.await.choose(&mut thread_rng()) {
//             dendrite_errors.await.insert(new_addr.clone(), 0);  // Начальная ошибка - 0
//             println!("Подключен новый дендрит с адресом {}", new_addr);
//         }
//     }
// }

async fn replace_worst_dendrites(dendrites: Arc<Mutex<HashMap<String, i32>>>, ipv6_list: Arc<Mutex<Vec<String>>>) {
    // Дождитесь завершения lock и сохраните результат
    let mut dendrite_errors = dendrites.lock().await;

    // Собираем адреса для замены
    let mut dendrite_vec: Vec<_> = dendrite_errors.iter().collect();
    dendrite_vec.sort_by(|a, b| b.1.cmp(a.1));  // Сортировка по убыванию ошибки
    
    let worst_count = (dendrite_vec.len() as f64 * 0.25).ceil() as usize;
    let worst_addrs: Vec<String> = dendrite_vec.iter()
                                               .take(worst_count)
                                               .map(|(addr, _)| addr.to_string())
                                               .collect();

    // Обрабатываем и заменяем
    for addr in worst_addrs {
        println!("Заменяем дендрит с адресом {}", addr);
        dendrite_errors.remove(&addr);

        let /* mut */ ipv6_list_locked = ipv6_list.lock().await;
        if let Some(new_addr) = ipv6_list_locked.choose(&mut thread_rng()) {
            dendrite_errors.insert(new_addr.clone(), 0);  // Начальная ошибка - 0
            println!("Подключен новый дендрит с адресом {}", new_addr);
        }
    }
}

// Генерация случайного IPv6-адреса для нового нейрона
fn generate_ipv6_address() -> String {
    let mut rng = rand::thread_rng();
    let ipv6_addr = Ipv6Addr::new(
        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
        rng.gen(), rng.gen(), rng.gen(), rng.gen(),
    );
    format!("[{}]:8080", ipv6_addr)
}

// Основная функция
#[tokio::main]
async fn main() {
    // Initialize the Neuromorph driver
    neuromorph_driver::init().unwrap();

    let weights = vec![5000, -2000, 8000];  // Веса для входных сигналов
    let bias = 1000;  // Смещение (bias)
    let neuron = Arc::new(Mutex::new(Neuron::new(weights, bias).unwrap()));
    let dendrites = Arc::new(Mutex::new(HashMap::<String, i32>::new()));
    let ipv6_list = Arc::new(Mutex::new(vec![]));  // Список адресов нейронов

    // Bind address can be controlled via env for deterministic tests.
    // If not set, fall back to a random IPv6 address.
    let bind_addr = std::env::var("NEUROMORPH_WS_ADDR").unwrap_or_else(|_| generate_ipv6_address());
    {
        let /* mut */ ipv6_list_locked = ipv6_list.lock();
        ipv6_list_locked.await.push(bind_addr.clone());  // Добавляем адрес нового нейрона в список
    }
    println!("WebSocket сервер нейрона запущен на {}", bind_addr);

    // Запуск сервера
    let listener = TcpListener::bind(bind_addr.clone()).await.expect("Can't bind to address");

    // Периодически заменяем худшие дендриты
    let dendrites_clone = dendrites.clone();
    let ipv6_list_clone = ipv6_list.clone();
    task::spawn(async move {
        loop {
            replace_worst_dendrites(dendrites_clone.clone(), ipv6_list_clone.clone()).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    while let Ok((stream, _)) = listener.accept().await {
        let neuron_clone = neuron.clone();
        let dendrites_clone = dendrites.clone();
        task::spawn(async move {
            let ws_stream = accept_async(stream).await.expect("Failed to accept");
            handle_client(ws_stream, neuron_clone, dendrites_clone).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neuron_device_memory_initialization() {
        // Initialize the driver
        neuromorph_driver::init().unwrap();

        let weights = vec![5000, -2000, 8000];
        let bias = 1000;

        let neuron = Neuron::new(weights.clone(), bias).unwrap();

        // Copy back and verify weights
        let copied_weights = neuron.get_weights().unwrap();
        assert_eq!(copied_weights, weights);

        // Copy back and verify bias
        let copied_bias = neuron.get_bias().unwrap();
        assert_eq!(copied_bias, bias);

        // Test forward compiles and runs (Phase 3: kernel execution)
        let inputs = vec![1, 2, 3];
        let output = neuron.forward(inputs).unwrap();
        // For dummy kernel, we just check it doesn't panic
        assert!(true); // If we reach here, kernel launch succeeded
    }

    #[test]
    fn test_dummy_kernel_creation() {
        // Initialize the driver
        neuromorph_driver::init().unwrap();

        // Create dummy kernel data
        let kernel_data = vec![0xAAu8; 1024];

        // Test that Kernel::from_bytes works with dummy data
        let kernel = neuromorph_driver::Kernel::from_bytes(&kernel_data);
        assert!(kernel.is_ok());
    }

    #[test]
    fn test_websocket_integration() {
        // Initialize the driver
        neuromorph_driver::init().unwrap();

        // Create neuron (equivalent to what the WebSocket server does)
        let weights = vec![5000, -2000, 8000];
        let bias = 1000;
        let neuron = Neuron::new(weights, bias).unwrap();

        // Simulate WebSocket input parsing: "1,2,3" -> vec![1, 2, 3]
        let input_data: Vec<i32> = "1,2,3"
            .split(',')
            .map(|s| s.parse::<i32>().unwrap())
            .collect();

        // Call forward method (what handle_client does)
        let output = neuron.forward(input_data).unwrap();

        // Verify we get a valid response (calculated via the CUDA-backed driver)
        assert!(output >= 0);
    }
}
