#[derive(Debug)]
pub struct ProcedureKillCommand {
    pub url: String,
    pub branch: String,
}

#[derive(Debug)]
pub struct ProcedureKillResponse {
    pub close_code: u32,
}