use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone, Default)]
pub struct TerminationRequest {
    requested: Arc<(Mutex<bool>, Condvar)>,
}

impl TerminationRequest {
    pub fn terminate(&self) {
        let (mutex, condition_variable) = &*self.requested;
        {
            let mut is_requested = mutex.lock().unwrap();
            *is_requested = true;
        }
        condition_variable.notify_all();
    }

    pub fn is_requested(&self) -> bool {
        let (mutex, _) = &*self.requested;
        let is_requested = mutex.lock().unwrap();
        *is_requested
    }

    pub fn wait(&self) {
        let (mutex, condition_variable) = &*self.requested;
        let _termination = condition_variable
            .wait_while(mutex.lock().unwrap(), |is_requested| !*is_requested)
            .unwrap();
    }
}
