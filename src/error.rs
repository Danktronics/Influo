#[derive(Debug)]
pub enum ProcedureError {
    ChildKillFail,
    ChildEndMissingCloseCode
}