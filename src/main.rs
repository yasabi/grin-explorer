#[macro_use] extern crate rocket;
use rocket_dyn_templates::Template;
use rocket_dyn_templates::context;
use rocket::fs::FileServer;
use rocket::State;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use colored::Colorize;
use rocket::tokio;
use rocket::response::Redirect;
use either::Either;

mod worker;
mod requests;
mod data;

use crate::data::Dashboard;
use crate::data::Block;
use crate::data::Transactions;


// Rendering main (Dashboard) page.
#[get("/")]
fn index(dashboard: &State<Arc<Mutex<Dashboard>>>) -> Template {
    let data = dashboard.lock().unwrap();

    Template::render("index", context! {
        route: "index",
        node_ver: &data.node_ver,
        proto_ver: &data.proto_ver,
    })
}


// Rendering block list (Blocks) page.
#[get("/block_list")]
fn block_list() -> Template {
    Template::render("block_list", context! {
        route: "block_list",
    })
}


// Rendering block list starting with a specified height.
// [<--] and [-->] buttons at the bottom of the block list (Blocks) page.
#[get("/block_list/<input_height>")]
async fn block_list_by_height(input_height: &str) -> Template {
    let mut blocks = Vec::<Block>::new();
    // Store current latest height
    let mut height = 0;

    let _ = requests::get_block_list_by_height(&input_height, &mut blocks, &mut height).await;

    // Check if user's input doesn't overflow current height
    if blocks.is_empty() == false && blocks[0].height.is_empty() == false {
        let index = blocks[0].height.parse::<u64>().unwrap();

        if index >= height {
            Template::render("block_list", context! {
                route: "block_list",
            })
        } else {
            Template::render("block_list", context! {
                route: "block_list_by_height",
                index,
                blocks,
                height,
            })
        }
    } else {
        Template::render("block_list", context! {
            route: "block_list",
        })
    }
}


// Rendering page for a specified block (by height).
#[get("/block/<height>")]
async fn block_details_by_height(height: &str) -> Template {
    let mut block = Block::new();

    if height.is_empty() == false && height.chars().all(char::is_numeric) == true {
        let _ = requests::get_block_data(&height, &mut block).await;

        if block.height.is_empty() == false {
            return Template::render("block_details", context! {
                route: "block_details",
                block,
            });
        }
    }

    Template::render("error", context! {
        route: "error",
    })
}


// Rendering page for a specified block (by hash).
#[get("/hash/<hash>")]
async fn block_header_by_hash(hash: &str) -> Either<Template, Redirect> {
    let mut height = String::new();

    let _ = requests::get_block_header(&hash, &mut height).await;

    if hash.is_empty() == false {
        if height.is_empty() == false {
            return Either::Right(Redirect::to(uri!(block_details_by_height(height.as_str()))));
        }
    }

    return Either::Left(Template::render("error", context! {
        route: "error",
    }))
}


// Rendering page for a specified kernel.
#[get("/kernel/<kernel>")]
async fn kernel(kernel: &str) -> Either<Template, Redirect> {
    let mut height = String::new();

    let _ = requests::get_kernel(&kernel, &mut height).await;

    if kernel.is_empty() == false {
        if height.is_empty() == false {
            return Either::Right(Redirect::to(uri!(block_details_by_height(height.as_str()))));
        }
    }

    return Either::Left(Template::render("error", context! {
        route: "error",
    }))
}


// Handling search request.
#[post("/search", data="<input>")]
fn search(input: &str) -> Either<Template, Redirect> {
    // Trim 'search=' from the request data
    let input = &input[7..].to_lowercase();
    
    //Check if input is valid
    if input.is_empty() == false {
        if input.chars().all(|x| (x >= 'a' && x <= 'f') || (x >= '0' && x <= '9')) == true {

            // Block number
            if input.chars().all(char::is_numeric) == true {
                return Either::Right(Redirect::to(uri!(block_details_by_height(input))));

            // Block hash
            } else if input.len() == 64 {
                return Either::Right(Redirect::to(uri!(block_header_by_hash(input))));
            
            // Kernel hash
            } else if input.len() == 66 {
                return Either::Right(Redirect::to(uri!(kernel(input))));
            }
        }
    }
    
    Either::Left(Template::render("error", context! {
        route: "error",
    }))
}


// Start of HTMX routes.
#[get("/rpc/peers/inbound")]
fn peers_inbound(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.inbound.to_string()
}


#[get("/rpc/peers/outbound")]
fn peers_outbound(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.outbound.to_string()
}


#[get("/rpc/sync/status")]
fn sync_status(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.sync == "no_sync" {
        "Synced".to_string()
    } else {
        format!("Syncing ({})
                 <div class='spinner-grow spinner-grow-sm' role='status'>
                 <span class='visually-hidden'>Syncing...</span></div>", data.sync)
    }
}


#[get("/rpc/market/supply")]
fn market_supply(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("ツ {}", data.supply)
}


#[get("/rpc/market/soft_supply")]
fn soft_supply(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.supply.is_empty() == false {
        return format!("{} % ({}M/3150M)", data.soft_supply, &data.supply[..3]);
    }
    
    "3150M".to_string()
}


#[get("/rpc/inflation/rate")]
fn inflation_rate(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("{} %", data.inflation)
}


#[get("/rpc/market/volume_usd")]
fn volume_usd(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.volume_usd)
}


#[get("/rpc/market/volume_btc")]
fn volume_btc(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("₿ {}", data.volume_btc)
}


#[get("/rpc/price/usd")]
fn price_usd(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.price_usd)
}


#[get("/rpc/price/btc")]
fn price_btc(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data       = dashboard.lock().unwrap();
    let trim: &[_] = &['0', '.'];

    format!("{} sats", data.price_btc.trim_start_matches(trim))
}


#[get("/rpc/market/cap_usd")]
fn mcap_usd(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.cap_usd)
}


#[get("/rpc/market/cap_btc")]
fn mcap_btc(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("₿ {}", data.cap_btc)
}


#[get("/rpc/block/latest")]
fn latest_height(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.height.clone()
}


#[get("/rpc/block/time_since_last")]
fn last_block_age(blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return data[0].time.clone();
    }
    
    "".to_string()
}


#[get("/rpc/disk/usage")]
fn disk_usage(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("{} GB", data.disk_usage)
}


#[get("/rpc/network/hashrate")]
fn network_hashrate(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("{} KG/s", data.hashrate)
}


#[get("/rpc/mining/production_cost")]
fn production_cost(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    format!("$ {}", data.production_cost)
}


#[get("/rpc/mining/reward_ratio")]
fn reward_ratio(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.reward_ratio.is_empty() == false {
        let ratio = data.reward_ratio.parse::<f64>().unwrap();

        if ratio <= 1.0 {
            return format!("x{} <i class='bi bi-hand-thumbs-down'></i>", data.reward_ratio);
        } else if ratio < 2.0 {
            return format!("x{} <i class='bi bi-hand-thumbs-up'></i>", data.reward_ratio);
        } else if ratio < 3.0 {
            return format!("x{} <i class='bi bi-emoji-sunglasses'></i>", data.reward_ratio);
        } else if ratio >= 3.0 {
            return format!("x{} <i class='bi bi-rocket-takeoff'></i>", data.reward_ratio);
        }
    }

    data.reward_ratio.clone()
}


#[get("/rpc/network/difficulty")]
fn network_difficulty(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.difficulty.to_string()
}


#[get("/rpc/mempool/txns")]
fn mempool_txns(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.txns.to_string()
}


#[get("/rpc/mempool/stem")]
fn mempool_stem(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    data.stem.to_string()
}


#[get("/rpc/txns/count_1h")]
fn txns_count_1h(transactions: &State<Arc<Mutex<Transactions>>>) -> String {
    let data = transactions.lock().unwrap();

    format!("{}, ツ {}", data.period_1h, data.fees_1h)
}


#[get("/rpc/txns/count_24h")]
fn txns_count_24h(transactions: &State<Arc<Mutex<Transactions>>>) -> String {
    let data = transactions.lock().unwrap();

    format!("{}, ツ {}", data.period_24h, data.fees_24h)
}


#[get("/rpc/block/link?<count>")]
fn block_link(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return format!("<a href=/block/{} class='text-decoration-none'>{}</a>",
                       data[count].height, data[count].height);
    }

    "".to_string()
}


#[get("/rpc/block/link_color?<count>")]
fn block_link_color(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return format!("<a href=/block/{} class='text-decoration-none darkorange-text'>{}</a>",
                       data[count].height, data[count].height);
    }
    
    "".to_string()
}


#[get("/rpc/block/time?<count>")]
fn block_time(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return data[count].time.clone();
    }

    "".to_string()
}


#[get("/rpc/block/kernels?<count>")]
fn block_txns(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return data[count].ker_len.to_string();
    }

    "".to_string()
}


#[get("/rpc/block/inputs?<count>")]
fn block_inputs(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return data[count].in_len.to_string();
    }

    "".to_string()
}


#[get("/rpc/block/outputs?<count>")]
fn block_outputs(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return data[count].out_len.to_string();
    }

    "".to_string()
}


#[get("/rpc/block/fees?<count>")]
fn block_fees(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return format!("ツ {}", data[count].fees / 1000000000.0);
    }

    "".to_string()
}


#[get("/rpc/block/weight?<count>")]
fn block_weight(count: usize, blocks: &State<Arc<Mutex<Vec<Block>>>>) -> String {
    let data = blocks.lock().unwrap();

    if data.is_empty() == false {
        return format!("{} %", data[count].weight);
    }

    "".to_string()
}


#[get("/rpc/block_list/index")]
fn block_list_index(dashboard: &State<Arc<Mutex<Dashboard>>>) -> String {
    let data = dashboard.lock().unwrap();

    if data.height.is_empty() == false {
        return format!("<a class='text-decoration-none' href='/block_list/{}'>
                        <div class='col-sm'><h2><i class='bi bi-arrow-right-square'></i></h2></div>
                        </a>", data.height.parse::<u64>().unwrap() - 10);
    }

    "".to_string()
}
// End of HTMX backends.


// Main
#[rocket::main]
async fn main() {
    println!("{} Starting up Explorer.", "[ INFO    ]".cyan());
    
    let dash         = Arc::new(Mutex::new(Dashboard::new()));
    let dash_clone   = dash.clone();
    let blocks       = Arc::new(Mutex::new(Vec::<Block>::new()));
    let blocks_clone = blocks.clone();
    let txns         = Arc::new(Mutex::new(Transactions::new()));
    let txns_clone   = txns.clone();
    let mut ready    = false;

    // Starting the Worker
    tokio::spawn(async move {
        loop {
            let result = worker::run(dash_clone.clone(), blocks_clone.clone(),
                                     txns_clone.clone()).await;
            
            match result {
                Ok(_v)  => {
                    if ready == false {
                        ready = true;
                        println!("{} Explorer Ready.", "[ OK      ]".green());
                    }
                },
                Err(e) => {
                    ready = false;
                    println!("{} {}.", "[ ERROR   ]".red(), e);
                },
            }
            
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });
    
    println!("{} Starting up Rocket engine.", "[ INFO    ]".cyan());

    // Starting Rocket engine.
    let _ = rocket::build()
            .manage(dash)
            .manage(blocks)
            .manage(txns)
            .mount("/", routes![index, peers_inbound, peers_outbound, sync_status, market_supply,
                                inflation_rate, volume_usd, volume_btc, price_usd, price_btc,
                                mcap_usd, mcap_btc,latest_height, disk_usage, network_hashrate,
                                network_difficulty, mempool_txns, mempool_stem, txns_count_1h,
                                txns_count_24h, block_list, block_link, block_link_color,
                                block_time, block_txns, block_inputs, block_outputs, block_fees,
                                block_weight, block_details_by_height, block_header_by_hash,
                                soft_supply, production_cost, reward_ratio, last_block_age,
                                block_list_by_height, block_list_index, search, kernel])
            .mount("/static", FileServer::from("static"))
            .attach(Template::fairing())
            .launch()
            .await;
}
