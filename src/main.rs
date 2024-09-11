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

// Структура для нейрона
#[derive(Clone)]
struct Neuron {
    weights: Vec<i32>,  // Веса синапсов (целые числа)
    bias: i32,          // Смещение (bias)
}

impl Neuron {
    fn new(weights: Vec<i32>, bias: i32) -> Neuron {
        Neuron { weights, bias }
    }

    fn forward(&self, inputs: Vec<i32>) -> i32 {
        let mut sum: i32 = 0;
        for (i, &input) in inputs.iter().enumerate() {
            sum += input * self.weights[i];
        }
        sum += self.bias;
        sigmoid(sum)
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
                
                let neuron_clone = neuron.lock();
                let output = neuron_clone.await.forward(input_data.clone());
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
    let weights = vec![5000, -2000, 8000];  // Веса для входных сигналов
    let bias = 1000;  // Смещение (bias)
    let neuron = Arc::new(Mutex::new(Neuron::new(weights, bias)));
    let dendrites = Arc::new(Mutex::new(HashMap::new()));
    let ipv6_list = Arc::new(Mutex::new(vec![]));  // Список адресов нейронов

    // Генерация и запуск WebSocket сервера на случайном IPv6-адресе
    let new_ipv6_addr = generate_ipv6_address();
    {
        let /* mut */ ipv6_list_locked = ipv6_list.lock();
        ipv6_list_locked.await.push(new_ipv6_addr.clone());  // Добавляем адрес нового нейрона в список
    }
    println!("WebSocket сервер нейрона запущен на {}", new_ipv6_addr);
    
    // Запуск сервера
    let listener = TcpListener::bind(new_ipv6_addr.clone()).await.expect("Can't bind to address");

    while let Ok((stream, _)) = listener.accept().await {
        let neuron_clone = neuron.clone();
        let dendrites_clone = dendrites.clone();
        task::spawn(async move {
            let ws_stream = accept_async(stream).await.expect("Failed to accept");
            handle_client(ws_stream, neuron_clone, dendrites_clone).await;
        });
    }

    // Периодически заменяем худшие дендриты
    let dendrites_clone = dendrites.clone();
    let ipv6_list_clone = ipv6_list.clone();
    task::spawn(async move {
        loop {
            replace_worst_dendrites(dendrites_clone.clone(), ipv6_list_clone.clone()).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });
}
