use crate::{MakeTokenPlaintext, TokenserverError};
use pyo3::{
    prelude::{IntoPy, PyErr, PyModule, PyObject, Python},
    types::IntoPyDict,
};

pub struct PyTokenlib {}
impl IntoPy<PyObject> for MakeTokenPlaintext {
    fn into_py(self, py: Python<'_>) -> PyObject {
        let dict = [
            ("node", self.node),
            ("fxa_kid", self.fxa_kid),
            ("fxa_uid", self.fxa_uid),
            ("hashed_device_id", self.hashed_device_id),
            ("hashed_fxa_uid", self.hashed_fxa_uid),
            ("tokenserver_origin", self.tokenserver_origin.to_string()),
        ]
        .into_py_dict(py);

        // These need to be set separately since they aren't strings, and
        // Rust doesn't support heterogeneous arrays
        dict.set_item("expires", self.expires).unwrap();
        dict.set_item("uid", self.uid).unwrap();

        dict.into()
    }
}
impl PyTokenlib {
    pub fn get_token_and_derived_secret(
        plaintext: MakeTokenPlaintext,
        shared_secret: &str,
    ) -> Result<(String, String), TokenserverError> {
        Python::with_gil(|py| {
            // `import tokenlib`
            let module = PyModule::import(py, "tokenlib").map_err(|e| {
                e.print_and_set_sys_last_vars(py);
                e
            })?;
            // `kwargs = { 'secret': shared_secret }`
            let kwargs = [("secret", shared_secret)].into_py_dict(py);
            // `token = tokenlib.make_token(plaintext, **kwargs)`
            let token = module
                .getattr("make_token")?
                .call((plaintext,), Some(kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .and_then(|x| x.extract())?;
            // `derived_secret = tokenlib.get_derived_secret(token, **kwargs)`
            let derived_secret = module
                .getattr("get_derived_secret")?
                .call((&token,), Some(kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .and_then(|x| x.extract())?;
            // `return (token, derived_secret)`
            Ok((token, derived_secret))
        })
        .map_err(pyerr_to_tokenserver_error)
    }
}

fn pyerr_to_tokenserver_error(e: PyErr) -> TokenserverError {
    TokenserverError {
        context: e.to_string(),
        ..TokenserverError::internal_error()
    }
}
