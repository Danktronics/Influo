use failure::{Error, err_msg};

pub mod procedure;

pub struct Project {
    pub url: String,
    pub procedures: Vec<Procedure>,
}

impl Project {
    pub fn new(raw_url: Value, raw_procedures: Value) -> Result<Project, Error> {
        if !raw_url.is_string() {
            return err_msg("URL is invalid");
        }
        let url: &String = raw_url.as_str().unwrap();

        if !raw_procedures.is_array() {
            return err_msg("Procedures is not an array");
        }
        let raw_procedures_array: &Vec<Value> = raw_procedures.as_array();
        let procedures: Vec<Procedure> = Vec::new();
        for raw_procedure in raw_procedures_array {
            procedures.push(Procedure::new(raw_procedures_array));
        }

        Project {
            url: url,
            procedures: procedures,
        }
    }
}