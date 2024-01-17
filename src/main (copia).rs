// Importación de macros y dependencias necesarias
#[macro_use] extern crate rocket;
use rocket::State;
use rocket::response::{self, Responder, Response};
use rocket::http::ContentType;
use rocket::fs::FileServer;
use rocket::fs::relative;
use rocket::fs::NamedFile;
//use rocket::http::uri::Path;
use bitcoincore_rpc::{Auth, Client, RpcApi, Error as BitcoinRpcError};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}, thread, time::Duration};
use std::path::Path;
//use std::io::prelude::*; // Para escribir en fichero
//use serde_json::{json, Value};
use serde_json::Value;

const SLEEP_TIME: u64 = 5;
const NUM_TX_PROCE: u64 = 100000;

// const USER:&str = "tu usuario";
// const PWS:&str  = "tu password";
const USER:&str = "userX";
const PWS:&str  = "wsx";


/// Estructura para representar una transacción padre-hijos e hijos - nietos
struct SeparatedTxGraph {
    // Relaciones padre -> hijos
    parent_child_edges: HashMap<String, HashSet<String>>,
    // Relaciones hijo -> nietos
    child_grandchild_edges: HashMap<String, HashSet<String>>,
}

// Implementación de la estructura SeparatedTxGraph
impl SeparatedTxGraph {

    // Constructor para crear un nuevo grafo de transacciones separado vacío
    fn new() -> SeparatedTxGraph {
        SeparatedTxGraph {
            parent_child_edges: HashMap::new(),
            child_grandchild_edges: HashMap::new(),
        }
    }

    // Función para agregar una relación padre-hijo entre dos transacciones
    fn add_parent_child_edges(&mut self, parent_id: String, child_id: String) {
        self.parent_child_edges.entry(parent_id).or_default().insert(child_id);
    }

    // Función para agregar una relación hijo-nieto entre dos transacciones
    fn add_child_grandchild_edges(&mut self, child_id: String, grandchild_id: String) {
        self.child_grandchild_edges.entry(child_id).or_default().insert(grandchild_id);
    }

    // Función para limpiar el grafo de transacciones,
    // eliminando aquellas que ya no están en la mempool
    fn clean_separated_tx_graph(&mut self, mempool_txs: &HashSet<String>) {

        self.parent_child_edges.retain(|tx_id, _| mempool_txs.contains(tx_id));
        self.child_grandchild_edges.retain(|tx_id, _| mempool_txs.contains(tx_id));
    }

    // Función para obtener los descendientes de una transacción
    // fn get_descendants(&self, tx_id: &str) -> HashSet<String> {
    //     let mut descendants = HashSet::new();
    //     if let Some(children) = self.parent_child_edges.get(tx_id) {
    //         for child in children {
    //             descendants.insert(child.clone());
    //             if let Some(grandchildren) = self.child_grandchild_edges.get(child) {
    //                 for grandchild in grandchildren {
    //                     descendants.insert(grandchild.clone());
    //                 }
    //             }
    //         }
    //     }
    //     descendants
    // }
}

// Estructura para manejar contenido HTML como respuesta
pub struct HtmlContent(String);
// Implementación del trait Responder para HtmlContent, permitiendo su uso como respuesta HTTP
impl<'r> Responder<'r, 'static> for HtmlContent {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> response::Result<'static> {
        Response::build()
            .header(ContentType::HTML)
            .sized_body(self.0.len(), std::io::Cursor::new(self.0))
            .ok()
    }
}

#[get("/script.js")]
async fn script_js() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/script.js")).await.ok()
}

#[get("/index")]
fn get_index() -> HtmlContent {

    let html_output = format!(
        "<!DOCTYPE html>
         <html>
             
             <head>
                 <script src='script.js'></script>
             </head>
             <body>
                 
                <!-- <h1>Resultado de la API</h1> -->
                <div id='apiResult'>Cargando...</div>
             </body>
             
         </html>" 
     );
 
     HtmlContent(html_output)
}


// Ruta del servidor web para obtener las transacciones descendientes en formato HTML
#[get("/get_descen_html")]
fn get_descen_html( separated_graph: &State<Arc<Mutex<SeparatedTxGraph>>>) -> HtmlContent {
    let separated_graph = separated_graph.lock().unwrap();
    
    let mut conta_padre ;
    let mut conta_hijo ;
    let mut conta_nieto ;
    let mut conta_todo = 0;

    // Generando contenido HTML con las transacciones
    let mut transactions = String::new();
    // transactions.push_str("<!DOCTYPE html>");
    transactions.push_str("<h1>Txs de la Mempool</h1>");
    // Transacciones separadas padre - padres
    let conta_tot_padre  = separated_graph.parent_child_edges.len() ;
    transactions.push_str(&format!("<h3> Total transacciones padre: {} </h3>", conta_tot_padre));

    transactions.push_str("<style> .tx-padre { color: black; } .tx-hijo { color: green; } .tx-nieto { color: blue; } </style>");

    // Iteramos sobre las transacciones padre separated_graph.parent_child_edges
    conta_padre = 1;
    for (parent_id, children) in separated_graph.parent_child_edges.iter() {
        transactions.push_str(&format!("<p class='tx-padre'> {}:Tx padre: {} </p>",conta_padre, parent_id));
        conta_todo += 1;
        conta_padre += 1;

        // Iteramos sobre las transacciones hijo children
        conta_hijo = 1;
        for child_id in children {
            transactions.push_str(&format!("<p class='tx-hijo'>&nbsp;&nbsp; {}:Tx hijo: {:?}</p>",conta_hijo, child_id));
            conta_todo += 1;
            conta_hijo += 1;

            // Iteramos sobre las transacciones nieto separated_graph.child_grandchild_edges
            if let Some(grandchildrens) = separated_graph.child_grandchild_edges.get(child_id) {
                conta_nieto = 1;
                for grandchildren in grandchildrens {
                    transactions.push_str(&format!("<p class='tx-nieto'>&nbsp;&nbsp;&nbsp;&nbsp; {}:Tx nieto: {:?}</p>",conta_nieto, grandchildren));
                    conta_todo += 1;
                    conta_nieto += 1;
                }
            }
        }
    }
    
    transactions.push_str(&format!("\n<p > Total líneas listado  {} : </p>", conta_todo));

    // Empaquetando el contenido HTML como una respuesta
    let html_output = format!("<html><body>{}</body></html>", transactions);
    HtmlContent(html_output)

}


// Función principal para lanzar el servidor Rocket
#[launch]
fn rocket() -> _ {
  
    // Inicializando el grafo de transacciones y su versión compartida entre hilos
    let separated_graph = Arc::new(Mutex::new(SeparatedTxGraph::new()));
    let separated_graph_clone = Arc::clone(&separated_graph);

    // Creando un hilo para actualizar el grafo periódicamente
    thread::spawn(move || {

        // Iniciar contador de tiempo
        let start = std::time::Instant::now();

        // Conexión con el nodo Bitcoin Core
        let rpc_url  = "http://localhost:8332";
        let rpc_auth = Auth::UserPass(USER.to_string(), PWS.to_string());
        let client = Client::new(rpc_url, rpc_auth).expect("Error to connect Bitcoin Core");


        // Parte primera: procesar todas las transacciones de la mempool
        let mut mempool_txs = get_raw_mempool(&client).expect("Error to get mempool transactions");
        
        println!("\nESPERE, PROCESANDO TODA LA MEMPOOL (este proceso puede tardar unos minutos)\n");
        println!("=> Txs mempool: {}", mempool_txs.len());
        
        get_graph(&mempool_txs, &client, &separated_graph_clone);

        // Calcular tiempo transcurrido en minutos y segundos desde start
        let duration = start.elapsed();
        let minutes = duration.as_secs() / 60;
        let seconds = duration.as_secs() % 60;
        let miliseconds = duration.subsec_millis();
        let velocity = mempool_txs.len() as f64 / (seconds as f64 + miliseconds as f64 / 1000.0);
        // Formatea la velocidad a 1 decimal
        let velocity = format!("{:.1}", velocity);

        println!("Procesadas todas las txs de la mempool: {}m {}s velocidad: {} Txs/s ", minutes, seconds, velocity);
        println!("=> Txs separadas padre: {}", separated_graph_clone.lock().unwrap().parent_child_edges.len());
        println!("=> Txs separadas hijo: {}\n", separated_graph_clone.lock().unwrap().child_grandchild_edges.len());
        println!("YA PUEDES HACER PETICIONES VIA WEB.\n\n");

        // Bucle infinito para procesar las transacciones nuevas que llegan a la mempool
        loop {

            // Iniciar contador de tiempo
            let start = std::time::Instant::now();

            // Obteniendo las transacciones nuevas de la mempool 
            // (mempool_news_txs - mempool_txs)
            let mempool_now = get_raw_mempool(&client).expect("Error al obtener transacciones del mempool");
            let mut mempool_new_txs = HashSet::new();
            for hash_tx in mempool_now.clone() {
                if !mempool_txs.contains(&hash_tx) {
                    mempool_new_txs.insert(hash_tx);
                }
            }

            mempool_txs = mempool_now;

            // Procesar las transacciones nuevas de la mempool para actualizar el grafo
            get_graph(&mempool_new_txs, &client, &separated_graph_clone);

            // Eliminamos del grafo las transacciones que ya no están en la mempool
            //graph_clone.lock().unwrap().clean_transactions(&mempool_txs);
            separated_graph_clone.lock().unwrap().clean_separated_tx_graph(&mempool_txs);

            //println!("=> Txs graf_clone: {}", graph_clone.lock().unwrap().edges.len());
            println!("=> Txs padre - hijos: {}", separated_graph_clone.lock().unwrap().parent_child_edges.len());
            println!("=> Txs hijo - nietos: {}", separated_graph_clone.lock().unwrap().child_grandchild_edges.len());
     
            // Calcular tiempo transcurrido en minutos y segundos desde start
            let duration = start.elapsed();
            let seconds = duration.as_secs() % 60;
            let miliseconds = duration.subsec_millis();
            let velocity = mempool_new_txs.len() as f64 / (seconds as f64 + miliseconds as f64 / 1000.0);
            // Formatea la velocidad a 1 decimal
            let velocity = format!("{:.1}", velocity);
            println!("Procesadas {} Txs nuevas: {}s {}ms velocidad: {} Txs/s \n", mempool_new_txs.len(), seconds, miliseconds, velocity);


            thread::sleep(Duration::from_secs(SLEEP_TIME));
        }
    });

    // Configurando el servidor Rocket con la ruta definida
    rocket::build()
        .manage(separated_graph)
        .mount("/", routes!(get_descen_html, get_index, script_js))
        .mount("/static", FileServer::from(relative!("static")))

}

// Función para obtener el grafo de transacciones
fn get_graph(mempool_txs: &HashSet<String>, client: &Client, 
             separated_graph: &Arc<Mutex<SeparatedTxGraph>>) {

    //let mut graph = graph.lock().unwrap();
    let mut separated_graph = separated_graph.lock().unwrap();

    let mut num_txs = 0;

    // Iterando sobre todas las transacciones de la mempool
    for hash_tx in mempool_txs {

        // Obtener los descendientes de la transacción actual (hijos)
        let descendants = get_mempool_descendants(client, hash_tx).unwrap_or_else(|_| vec![]);
        for desc_tx in descendants {
            // Procesar solamente las primeras NUM_TX_PROCE transacciones de la mempool
            num_txs += 1;
            if num_txs > NUM_TX_PROCE {
                break;
            }
            
            separated_graph.add_parent_child_edges(hash_tx.clone(), desc_tx.clone());
            
            // Obtener los descendientes de los descendientes (nietos)
            let desc_descendants = get_mempool_descendants(client, &desc_tx).unwrap_or_else(|_| vec![]);
            for desc_desc_tx in desc_descendants {
                
                separated_graph.add_child_grandchild_edges(desc_tx.clone(), desc_desc_tx.clone());
            }
        } 
    }

    // Iterar separated_graph.child_grandchild_edges  para eliminar de parent_child_edges 
    // los padres que están en child_grandchild_edges como hijos
    for (child_id, _grandchildren) in separated_graph.child_grandchild_edges.clone() {
        // Si child_id está en separated_graph.parent_child_edges 
        // eliminar child_id de separated_graph.parent_child_edges
        if separated_graph.parent_child_edges.contains_key(&child_id) {
            separated_graph.parent_child_edges.remove(&child_id);
        }

    }

}

fn get_mempool_descendants(client: &Client, txid: &str) -> bitcoincore_rpc::Result<Vec<String>> {
    match client.call("getmempooldescendants", &[Value::String(txid.to_string())]){
        Ok(descendants) => Ok(descendants),
        Err(e) => Err(e)
    }
}

fn get_raw_mempool(client: &Client) -> Result<HashSet<String>, BitcoinRpcError> {
    match client.call::<Vec<String>>("getrawmempool", &[Value::Bool(false)]) {
        Ok(mempool_txids) => {
            let txids: HashSet<String> = mempool_txids.into_iter().collect();
            Ok(txids)
        },
        Err(e) => Err(e)
    }
}




