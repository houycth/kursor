#![windows_subsystem = "windows"]


use std::{collections::HashSet, sync::{atomic::{AtomicBool, AtomicU64, Ordering}, Arc}};
use tokio::{sync::RwLock, time};
use device_query::DeviceState;
use rdev::{EventType, Key, Button, simulate, SimulateError};
use once_cell::sync::{Lazy, OnceCell};


// to flag process on or not.
static KURSOR_ON: AtomicBool = AtomicBool::new(false);
static MOVE_LOOP_ON: AtomicBool = AtomicBool::new(false);

static MOUSE_MOVE_STEP_Y: AtomicU64 = AtomicU64::new(8);
static MOUSE_MOVE_STEP_X: AtomicU64 = AtomicU64::new(16);

static SWITCH_KEY: Lazy<HashSet<Key>> = Lazy::new(|| {HashSet::from([Key::F7])});
static DIRECTION_KEYS: Lazy<HashSet<Key>> = Lazy::new(|| {HashSet::from([Key::KeyW, Key::KeyA, Key::KeyS, Key::KeyD])});
static OPERATION_KEYS: Lazy<HashSet<Key>> = Lazy::new(|| {HashSet::from([Key::KeyI, Key::KeyK, Key::KeyJ, Key::KeyL, Key::KeyQ, Key::KeyE])});

static KEY_PRESSED: Lazy<RwLock<KeyPressed>> = Lazy::new(|| {RwLock::new(KeyPressed {key_a: false, key_w: false, key_d: false, key_s: false})});

static TOKIO_RT: OnceCell<Arc<tokio::runtime::Runtime>> = OnceCell::new();


struct KeyPressed {
    key_w: bool,
    key_a: bool,
    key_s: bool,
    key_d: bool,
}


fn is_binding_key(key: &Key) -> bool {
    DIRECTION_KEYS.contains(&key) || OPERATION_KEYS.contains(&key) || SWITCH_KEY.contains(&key)
}

fn is_direction_key(key: &Key) -> bool {
    DIRECTION_KEYS.contains(&key)
}

fn is_operation_key(key: &Key) -> bool {
    OPERATION_KEYS.contains(&key)
}

fn is_switch_key(key: &Key) -> bool {
    SWITCH_KEY.contains(&key)
}

fn emit_event(evt: &EventType) {
    match simulate(evt) {
        Ok(()) => (),
        Err(SimulateError) => {
            // println!("We could not send {:?}", evt);
        }
    }
}


async fn move_loop() {
    if MOVE_LOOP_ON.load(Ordering::Relaxed) {
        return;
    }

    // println!("Command loop started!");
    let device_state = DeviceState::new();
    loop {
        let step_y = MOUSE_MOVE_STEP_Y.load(Ordering::Relaxed) as f64;
        let step_x = MOUSE_MOVE_STEP_X.load(Ordering::Relaxed) as f64;
        let kursor_on = KURSOR_ON.load(Ordering::Relaxed);
        if !kursor_on {
            // println!("Command loop stopped!");
            MOVE_LOOP_ON.store(false, Ordering::Relaxed);
            break
        };

        let mouse_state = device_state.query_pointer();
        let (mouse_x, mouse_y) = mouse_state.coords;

        // println!("Mouse X: {mouse_x}, Y: {mouse_y}");
        let mut mouse_x = mouse_x as f64;
        let mut mouse_y = mouse_y as f64;

        let key_pressed = KEY_PRESSED.read().await;
        if key_pressed.key_a {
            mouse_x = mouse_x - step_x;
        }
        if key_pressed.key_d {
            mouse_x = mouse_x + step_x;
        }
        if key_pressed.key_w {
            mouse_y = mouse_y - step_y;
        }
        if key_pressed.key_s {
            mouse_y = mouse_y + step_y;
        }

        emit_event(&EventType::MouseMove { x: mouse_x, y: mouse_y });
        // TODO 循环时间可以设置为屏幕刷新时间
        time::sleep(time::Duration::from_millis(7)).await;
    }
}

fn start_ctrl_thread() {
    let rt = TOKIO_RT.get().unwrap();
    rt.spawn(move_loop());
}

// 这个函数只修改状态，在单独线程中执行命令
async fn direction_key_press(key: &Key) {
    let mut key_pressed = KEY_PRESSED.write().await;
    match key {
        Key::KeyA => {
            key_pressed.key_a = true;
        }
        Key::KeyD => {
            key_pressed.key_d = true;
        }
        Key::KeyW => {
            key_pressed.key_w = true;
        }
        Key::KeyS => {
            key_pressed.key_s = true;
        }
        _ => {}
    }
}

async fn direction_key_release(key: &Key) {
    let mut key_pressed = KEY_PRESSED.write().await;
    match key {
        Key::KeyA => {
            key_pressed.key_a = false;
        }
        Key::KeyD => {
            key_pressed.key_d = false;
        }
        Key::KeyW => {
            key_pressed.key_w = false;
        }
        Key::KeyS => {
            key_pressed.key_s = false;
        }
        _ => {}
    }
}

fn operation_key_press(key: &Key) {
    match key {
        Key::KeyI => {
            emit_event(&EventType::Wheel { delta_x: 0, delta_y: 1 });
        }
        Key::KeyK => {
            emit_event(&EventType::Wheel { delta_x: 0, delta_y: -1 });
        }
        Key::KeyL => {
            emit_event(&EventType::ButtonPress(Button::Right));
        }
        Key::KeyJ => {
            emit_event(&EventType::ButtonPress(Button::Left));
        }
        Key::KeyQ => {
            let step_x = MOUSE_MOVE_STEP_X.load(Ordering::Relaxed);
            let step_y = MOUSE_MOVE_STEP_Y.load(Ordering::Relaxed);
            let mut new_x = step_x - 2;
            let mut new_y = step_y - 2;
            if new_y < 2 {
                new_y = 2;
                new_x = step_x;
            }
            MOUSE_MOVE_STEP_X.store(new_x, Ordering::Relaxed);
            MOUSE_MOVE_STEP_Y.store(new_y, Ordering::Relaxed);
            // println!("Change mouse move step to: x -> {new_x}; y -> {new_y}");
        }
        Key::KeyE => {
            let step_x = MOUSE_MOVE_STEP_X.load(Ordering::Relaxed);
            let step_y = MOUSE_MOVE_STEP_Y.load(Ordering::Relaxed);
            let new_x = step_x + 2;
            let new_y = step_y + 2;
            MOUSE_MOVE_STEP_X.store(new_x, Ordering::Relaxed);
            MOUSE_MOVE_STEP_Y.store(new_y, Ordering::Relaxed);
            // println!("Change mouse move step to: x -> {new_x}; y -> {new_y}");
        }
        _ => {}
    }    
}

fn operation_key_release(key: &Key) {
    match key {
        Key::KeyL => {
            emit_event(&EventType::ButtonRelease(Button::Right));
        }
        Key::KeyJ => {
            emit_event(&EventType::ButtonRelease(Button::Left));
        }
        _ => {}
    }  
}



fn main() {
    TOKIO_RT.set(Arc::new(tokio::runtime::Runtime::new().expect("Failed to create a new tokio runtime."))).unwrap();

    // 拦截绑定的按键
    let result = rdev::grab(move |ev| match ev.event_type {
        EventType::KeyPress(key) => {

            // toggle kursor status
            if is_switch_key(&key) {
                let kursor_on = KURSOR_ON.load(Ordering::Relaxed);
                KURSOR_ON.store(!kursor_on, Ordering::Relaxed);
                // println!("Switch Kursor status: {}", !kursor_on);

                if KURSOR_ON.load(Ordering::Relaxed) {
                    start_ctrl_thread();
                }

                return Some(ev);
            }

            // Do nothing if kursor is off or key is not containded in binding keys.
            if !KURSOR_ON.load(Ordering::Relaxed) || !is_binding_key(&key) {
                Some(ev)
            } else {
                // println!("Keyboard key press: {:#?}", key);

                let rt = TOKIO_RT.get().unwrap();
                rt.spawn(async move {
                    if is_direction_key(&key) {
                        direction_key_press(&key).await
                    }

                    if is_operation_key(&key) {
                        operation_key_press(&key);
                    }
                });

                // To prevent default event.
                None
            }
        }
        EventType::KeyRelease(key) => {
            let kursor_on = KURSOR_ON.load(Ordering::Relaxed);

            if !kursor_on || !is_binding_key(&key) {
                Some(ev)
            } else {
                // println!("Keyboard key release: {:#?}", key);

                let rt = TOKIO_RT.get().unwrap();
                rt.spawn(async move {
                    if is_direction_key(&key) {
                        direction_key_release(&key).await;
                    }

                    if is_operation_key(&key) {
                        operation_key_release(&key);
                    }
                });

                None
            }
        }
        _ => Some(ev),
    });

    if let Err(e) =  result {
        println!("拦截出错：{e:?}");
    }
}
