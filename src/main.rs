use std::thread;
use std::time::Duration;

use glib::{clone, Continue, MainContext, PRIORITY_DEFAULT};
use gtk::glib::timeout_future_seconds;
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button};

const APP_ID: &str = "event_loop_test";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    let mut func_vec:Vec<&dyn Fn(&Application)> = Vec::new();

    func_vec.push(&build_app_with_stuck_behavior);
    func_vec.push(&build_app_with_new_thread);
    func_vec.push(&build_app_with_new_thead_and_button_disable);
    func_vec.push(&build_app_with_async_button);

    for func in func_vec{
        app.connect_activate(func);
    }

    app.run()
}

/// This is the synchronus way to make a button
/// For short and simple task, it should be fine;
/// however, if there is a heavy task that takes a few seconds,
/// the button can freeze the application because
/// the main loop is handling the main logic of the button,
/// so the graphic can't be rendered
///
fn build_app_with_stuck_behavior( app: &Application ) {

    let button = build_button();

    // send a click signal to wait; this will freeze the windows
    button.connect_clicked(move |_|{
        // send a click signal to wait; this will freeze the windows
        let a_few_moments_later = Duration::from_secs(10);
        std::thread::sleep(a_few_moments_later);
    });

    present_button_interface(app, &button, "Stuck Synchronus Call");
}

/// For that issue, we can always start a new thread to 
/// relocate the main logic to somewhere else instead of 
/// blocking the main loop.
/// 
/// This is not a perfect solution though, as users can keep on pressing the button,
/// and submitting multiple threads that might introduces unwanted behaviors.
/// 
fn build_app_with_new_thread( app: &Application ) {

    let button = build_button();

    // we can always spawn a new thread to put away those heavy processes
    button.connect_clicked(move |_|{
        // spawn a new thread:
        thread::spawn(
            move | |{
                let a_few_moments_later = Duration::from_secs(10);
                std::thread::sleep(a_few_moments_later);
            }
        );
    });

    present_button_interface(app, &button, "Button with New Thread");
}

/// Hence, we need a MainContext.
/// To do this, we need to send some signal (boolean in this cause) to the main event loop,
/// then use the attach function from the receiver to change the button status
/// 
/// @default-return Continue(false) use for preventing callback when the weak reference fails to be updated
/// Otherwise, we call the closure to maniplute the clickability of the button.
/// 
fn build_app_with_new_thead_and_button_disable( app: &Application ){

    // however, nothing can prevent from user spawning new thread indefinitely. 
    // to limit user able to spawn a single thread at a time, a channel is needed
    let (sender, receiver) = MainContext::channel(PRIORITY_DEFAULT);
    let button = build_button();

    button.connect_clicked(move |_|{
            let sender = sender.clone();
            // here is where the thread spawned
            thread::spawn(move | |{
                // deactivate the button (similar to disable in html + js) until the wait has ended
                sender.send(false).expect("Errer during channel send.");
                let a_few_moments_later = Duration::from_secs(10);
                std::thread::sleep(a_few_moments_later);

                // enable the button again
                sender.send(true).expect("Errer during channel send.");
            });
        }
    );

    // The main loop executes the closure as soon as it receives the message
    receiver.attach(
        None,
        clone!(
            @weak button => @default-return Continue(false), // <- prevent to call the closure when the weak reference update fails
                move |enable_button| {                       // <- normally call this closure
                    button.set_sensitive(enable_button);
                    Continue(true)
                }
        ),
    );

    present_button_interface(&app, &button, "Button with New Thread and Disable (sensitive) behavior");

}

/// It is possible use MainContext in an async fashion by using spawn_local function,
/// to prevent the process from freezing by a button, without the additional thread.
/// 
fn build_app_with_async_button( app: &Application ) {
    
    let button = build_button();

    // Alternativelly, we can use MainContext with single thread process, using async
    button.connect_clicked(move |button|{
        let main_context = MainContext::default();
        // run the async block
        main_context.spawn_local(clone!{
            @weak button => async move {
                // disable the button using set_sensitive
                button.set_sensitive(false);
                timeout_future_seconds(10).await;
                // enable the button
                button.set_sensitive(true);
            }
        });
    });

    present_button_interface(app, &button, "Async Button");
}

/// The following are the refactored function that are used for reducing repatitive code
fn build_button() -> Button{
    // button for demonstracting the event loop
    let button = Button::builder()
        .label("Press and wait eternally")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    button
}

fn present_button_interface(app: &Application, button: &Button, title: &str){
    // craete all the remaining structure of the app
    let window = ApplicationWindow::builder()
        .application(app)
        .title("GTK4 Event Tutorial - ".to_owned() + title)
        .child(button)
        .build();

    window.present();
}