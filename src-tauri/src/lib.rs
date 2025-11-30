use tauri::window::Color;
use tauri::{command, AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

use log::{info, warn};

use std::fmt::Write;
use std::sync::Mutex;
use std::time::Duration;

use tokio::time::sleep;

#[command]
fn greet(name: &str) -> String {
    info!("Greet command invoked with name: {}", name);
    format!("Hello, {}! You've been greeted from Rust!", name)
}

static URLS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static INDEX: Mutex<usize> = Mutex::new(0);
static PAGE_CHANGE_TIMESTAMP: Mutex<i64> = Mutex::new(0);
static ERROR_LOOP_STARTED: Mutex<bool> = Mutex::new(false);

#[allow(dead_code)]
#[command]
fn get_page_change_timestamp() -> i64 {
    *PAGE_CHANGE_TIMESTAMP.lock().unwrap()
}

#[allow(dead_code)]
#[command]
fn set_page_change_timestamp(timestamp: i64) {
    let mut page_change_timestamp = PAGE_CHANGE_TIMESTAMP.lock().unwrap();
    *page_change_timestamp = timestamp;
}

#[allow(dead_code)]
#[command]
fn create_list() -> String {
    let urls = URLS.lock().unwrap();
    let mut url_list_html = String::from(
        r##"
        <style>
        #url-list .pie-container {
            width: 14px;
            height: 14px;
            position: absolute;
            top: 0;
            left: 0;
            display: flex;
            align-items: center;
            justify-content: center;
            font-family: Arial, sans-serif;
            font-size: 1em;
            font-weight: bold;
            pointer-events: none;
        }
        #url-list {
            display: flex;
            flex-direction: column;
            gap: 8px;
            overflow: visible;
        }
        #url-list .url-dot-container {
            position: relative;
            display: flex;
            align-items: center;
            height: 20px;
        }
        #url-list .url-dot {
            width: 14px;
            height: 14px;
            background: #fff;
            border-radius: 50%;
            display: inline-block;
            cursor: pointer;
            border: 2px solid #2196f3;
            transition: box-shadow 0.2s;
            position: relative;
            z-index: 1;
            flex-shrink: 0;
        }
        #url-list .url-dot-container .url-tooltip {
            max-width: 260px;
            color: #fff;
            background: rgba(0, 0, 0, 0.5);
            text-align: left;
            border-radius: 4px;
            padding: 6px 10px;
            margin-left: 12px;
            white-space: pre-line;
            font-size: 13px;
            opacity: 0;
            visibility: hidden;
            transition: opacity 0.2s, visibility 0.2s;
            position: relative;
            left: 0;
            top: 0;
            pointer-events: none;
            z-index: 2;
            display: inline-block;
            word-break: break-all;
        }
        #url-list .url-dot:hover + .url-tooltip,
        #url-list .url-dot:focus + .url-tooltip {
            opacity: 1;
            visibility: visible;
        }
        #url-list .url-dot:hover {
            box-shadow: 0 0 0 3px #2196f3aa;
        }
        </style>
        <div id="url-list" style='position:fixed;top:0;left:0;padding:8px;z-index:9999;font-size:14px;max-width:300px;'>
    "##,
    );

    let current_index = INDEX.lock().unwrap();
    for (index, url) in urls.iter().enumerate() {
        let active = if *current_index == index {"active-dot"} else {""};
        let _ = write!(
            url_list_html,
            r##"<div class="url-dot-container">
    <a href="#" class="url-dot {}" tabindex="0" onclick="window.__TAURI_INTERNALS__.invoke('change_url', {{ index: {}, endTime: 0 }}); return false;">
        {}
    </a>
    <span class="url-tooltip">{}</span>
</div>"##,
            active,
            index,
            if active == "active-dot" {
                // Insert the SVG pie for the active dot
                r##"<span class="pie-container">
                    <svg width="14" height="14">
                        <circle cx="7" cy="7" r="6" stroke="#21f623" stroke-width="1.5" fill="none"/>
                        <path id="pie-fill-active" fill="#21f623" stroke="none"/>
                    </svg>
                </span>"##
            } else {
                ""
            },
            url
        );
    }
    url_list_html.push_str("</div>");
    url_list_html
}

#[command]
async fn create_window(app_handle: AppHandle, urls: Vec<String>) {
    info!("Creating a new window...");

    *URLS.lock().unwrap() = urls.clone();
    let webview_option = app_handle
        .get_webview_window("screen");
    if webview_option.is_some() {
        info!("Window with label 'screen' already exists, skipping creation.");
        webview_option.unwrap().reload().expect("Failed reload webview");
        return;
    };

    let inject_script = format!(
        r##"
        document.addEventListener('DOMContentLoaded', function() {{

        let container = document.createElement('div');
        container.id = 'url-list-shadow-container';
        document.body.insertAdjacentElement('beforebegin', container);
        let shadow = container.attachShadow({{ mode: 'open' }});

        let loadingOverlay = document.createElement('div');
        loadingOverlay.id = 'tauri-loading-overlay';
        loadingOverlay.style.cssText = `
            position: fixed;
            top: 0; left: 0; width: 100vw; height: 100vh;
            background: rgba(255,255,255,0.85);
            z-index: 999;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 2em;
            color: #2196f3;
            transition: opacity 0.2s;
            opacity: 0;
            pointer-events: none;
        `;
        loadingOverlay.innerHTML = '<div style="display:flex;flex-direction:column;align-items:center;"><svg width="48" height="48" viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="24" cy="24" r="20" stroke="#2196f3" stroke-width="4" stroke-linecap="round" stroke-dasharray="31.4 31.4" stroke-dashoffset="0"><animateTransform attributeName="transform" type="rotate" repeatCount="indefinite" dur="1s" values="0 24 24;360 24 24" keyTimes="0;1"/></circle></svg><span style="margin-top:12px;">Loading...</span></div>';
        shadow.appendChild(loadingOverlay);

        (async () => {{
            let list = await window.__TAURI_INTERNALS__.invoke('create_list');
            console.log('URL List:', list);
            shadow.innerHTML += list;
            const dots = shadow.querySelectorAll('.url-dot');
            for (let i = 0; i < dots.length; i++) {{
                const dot = dots[i];
                dot.addEventListener('click', (e) => {{
                    e.preventDefault();
                    window.__TAURI_INTERNALS__.invoke('change_url', {{ index : i, endTime: 0 }});
                }});
            }}
            const dot = shadow.querySelector('.active-dot');
            if (dot) {{
                const data = dot.getAttribute('data');
                dot.innerHTML = `
                    <span class="pie-container">
                        <svg width="14" height="14">
                            <circle cx="7" cy="7" r="6" stroke="#21f623" stroke-width="1.5" fill="none"/>
                            <path id="pie-fill-active" fill="#21f623" stroke="none"/>
                        </svg>
                    </span>
                `;
                function describeArc(cx, cy, r, percent) {{
                    const angle = percent / 100 * 360;
                    const radians = (angle - 90) * Math.PI / 180.0;
                    const x = cx + r * Math.cos(radians);
                    const y = cy + r * Math.sin(radians);
                    const largeArc = angle > 180 ? 1 : 0;
                    if (percent === 0) {{
                        return '';
                    }}
                    if (percent === 100) {{
                        return `M ${{cx}},${{cy}} m -${{r}},0 a ${{r}},${{r}} 0 1,0 ${{r*2}},0 a ${{r}},${{r}} 0 1,0 -${{r*2}},0`;
                    }}
                    return [
                        `M ${{cx}},${{cy}}`,
                        `L ${{cx}},${{cy - r}}`,
                        `A ${{r}},${{r}} 0 ${{largeArc}},1 ${{x}},${{y}}`,
                        'Z'
                    ].join(' ');
                }}
                function setProgress(percent) {{
                    const fillPath = shadow.getElementById('pie-fill-active');
                    if (fillPath) {{
                        fillPath.setAttribute('d', describeArc(7, 7, 6, percent));
                    }}
                }}
                let percent = 0;
                const target = 100;
                let startTime = Date.now();
                let endTime = await window.__TAURI_INTERNALS__.invoke('get_page_change_timestamp');
                const interval = setInterval(async () => {{
                    const newTime = await window.__TAURI_INTERNALS__.invoke('get_page_change_timestamp');
                    if (newTime !== endTime) {{
                        endTime = newTime;
                        startTime = Date.now();
                        setProgress(0);
                        return;
                    }}
                    const percent = Math.min(100, Math.floor(((Date.now() - startTime) / (endTime - startTime)) * 100));
                    setProgress(percent);
                    if (percent === 100) {{
                        clearInterval(interval);
                        window.__TAURI_INTERNALS__.invoke('keyup', {{key: 'ArrowRight'}});
                    }}
                }}, 100);
            }}
        }})().catch((e) => console.error('Error creating URL list:', e));
        }});
        window.addEventListener('click', () => {{
            window.__TAURI_INTERNALS__.invoke('reset_timer');
        }});
        window.addEventListener('mousemove', () => {{
            window.__TAURI_INTERNALS__.invoke('reset_timer');
        }});
        window.addEventListener('keyup', (e) => {{
            window.__TAURI_INTERNALS__.invoke('reset_timer');
            window.__TAURI_INTERNALS__.invoke('keyup', {{key: e.key}});
        }});
        "##,
    );

    let _window = WebviewWindowBuilder::new(
        &app_handle,
        "screen",
        WebviewUrl::External(
            Url::parse(&*urls[0]).expect("Invalid URL for the new window"),
        ),
    )
    .title("Screen")
    .background_color(Color(1, 1, 1, 255))
    .fullscreen(true)
    .visible(true)
    .inner_size(600.0, 800.0)
    .initialization_script(&inject_script)
    .build()
    .expect("Failed to create new window");

    info!(
        "New window created successfully with label: {}",
        _window.label()
    );

    let webview = _window
        .get_webview_window("screen")
        .expect("Failed to get webview for the new window");

    info!("Webview for the new window: {:?}", webview.url().unwrap());

    {
        let mut error_loop_started = ERROR_LOOP_STARTED.lock().unwrap();
        if *error_loop_started {
            info!("Error loop already started");
            return;
        }
        *error_loop_started = true;
    }

    info!("starting error loop");
    loop {
        sleep(Duration::from_secs(5)).await;
        if app_handle.get_webview_window("screen").is_none() {
            info!("screen window is closed, stopping loop.");
            *ERROR_LOOP_STARTED.lock().unwrap() = false;
            break;
        }
        let status = check_window(app_handle.clone()).await;
        if !status {
            warn!("Window is dead, moving to next URL");
            keyup(app_handle.clone(), "ArrowRight".to_string());
        }
    }
}

static WINDOW_LOCK: Mutex<bool> = Mutex::new(false);

#[command]
async fn check_window(app_handle: AppHandle) -> bool {
    let webview_option = app_handle.get_webview_window("screen");
    if webview_option.is_none() {
        info!("Webview with label 'screen' does not exist.");
        return false;
    }
    let webview = webview_option.expect("Failed to get webview for the new window");

    webview.eval("window.__TAURI_INTERNALS__.invoke('window_alive')")
        .expect("Failed to execute console log in webview");

    *WINDOW_LOCK.lock().unwrap() = false;

    // wait 5 seconds for a response
    sleep(Duration::from_secs(5)).await;

    *WINDOW_LOCK.lock().unwrap()
}

#[command]
fn window_alive() {
    *WINDOW_LOCK.lock().unwrap() = true;
}

#[command]
async fn change_url(app_handle: AppHandle, index: usize, end_time: i64) {
    let url = {
        let urls = URLS.lock().unwrap();
        if index >= urls.len() {
            info!("Index {} is out of bounds for URL list", index);
            return;
        }
        urls[index].clone()
    };

    let webview_option = app_handle.get_webview_window("screen");
    if webview_option.is_none() {
        info!("Webview with label 'screen' does not exist.");
        return;
    }

    let webview = webview_option
        .expect("Failed to get webview for the new window");

    // let webview_url = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| webview.url()));
    // if webview_url.is_ok() {
    //     let webview_url_result = webview_url.unwrap();
    //     if webview_url_result.is_ok() {
    //         info!("Webview URL: {:?}", webview_url_result.unwrap());
    //     } else {
    //         info!("Webview URL is error");
    //         webview.reload().expect("Failed to reload webview");
    //         return;
    //     }
    // } else {
    //     info!("Webview URL is error");
    //     webview.reload().expect("Failed to reload webview");
    //     return;
    // }

    webview.eval(r#"
        console.log('Showing loading overlay');
            container = document.getElementById('url-list-shadow-container');
            shadow = container.shadowRoot;
            aloadingOverlay = shadow.getElementById('tauri-loading-overlay');
            if (aloadingOverlay) {{
                aloadingOverlay.style.opacity = '1';
                aloadingOverlay.style.pointerEvents = 'auto';
            }}
        setTimeout(() => {{
            window.location.reload();
        }}, 100);
    "#)
        .expect("Failed to execute console log in webview");

    sleep(Duration::from_millis(200)).await;
    let parsed_url = url.parse().expect("Failed to parse URL");
    webview.navigate(parsed_url).expect("msg");

    webview
        .set_title(&format!("{}", url))
        .expect("Failed to set window title");

    app_handle.emit("change-index", index)
        .expect("Failed to emit change-index event");

    let mut page_open_timestamp = PAGE_CHANGE_TIMESTAMP.lock().unwrap();
    *page_open_timestamp = end_time;

    let mut index_lock = INDEX.lock().unwrap();
    *index_lock = index;

    //info!("Webview URL changed successfully to: {}", url);
}

#[allow(dead_code)]
#[command]
fn reset_timer(app_handle: AppHandle) {
    app_handle
        .emit("reset-timer", ())
        .expect("Failed to emit reset-timer event");
}

#[allow(dead_code)]
#[command]
fn keyup(app_handle: AppHandle, key: String) {
    // info!("Key up event for key: {}", key);
    app_handle
        .emit("keyup", key)
        .expect("Failed to emit keyup event");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            create_window,
            reset_timer,
            change_url,
            keyup,
            create_list,
            get_page_change_timestamp,
            set_page_change_timestamp,
            check_window,
            window_alive
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

