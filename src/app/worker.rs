use std::sync::mpsc;
use std::time::Duration;

pub(super) const ACTION_RESULT_POLL_INTERVAL: Duration = Duration::from_millis(24);

pub(super) fn spawn_worker_action<T, W, H>(work: W, mut on_result: H)
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
    H: FnMut(T) + 'static,
{
    let (tx, rx) = mpsc::channel::<T>();
    std::thread::spawn(move || {
        let result = work();
        let _ = tx.send(result);
    });

    gtk4::glib::timeout_add_local(ACTION_RESULT_POLL_INTERVAL, move || match rx.try_recv() {
        Ok(result) => {
            on_result(result);
            gtk4::glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => gtk4::glib::ControlFlow::Continue,
        Err(mpsc::TryRecvError::Disconnected) => gtk4::glib::ControlFlow::Break,
    });
}
