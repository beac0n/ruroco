use std::error::Error;

pub fn run(start: String, stop: String, sleep: u8) -> Result<(), Box<dyn Error>> {
    // TODO:
    //   - open IPC socket
    //   - wait for incoming command execution requests
    //   - execute start
    //   - wait sleep seconds
    //   - execute stop
    Ok(())
}