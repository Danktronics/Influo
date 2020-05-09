#[derive(Clone, Debug)]
pub enum Command {
    KillProcedure,
    // RetrieveLogs,
}

#[derive(Clone, Debug)]
pub enum Response {
    // KilledProcedure(i32), // Close Code
    // Logs(Vec<String>), // Line separated logs
}
