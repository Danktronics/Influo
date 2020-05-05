#[derive(Debug)]
pub enum Command {
    KillProcedure,
    RetrieveLogs,
}

#[derive(Debug)]
pub enum Response {
    KilledProcedure(i32), // Close Code
    Logs(Vec<String>), // Line separated logs
}