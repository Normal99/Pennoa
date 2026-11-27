//! Bridge to the [Giac](https://www-fourier.univ-grenoble-alpes.fr/~parisse/giac.html)
//! computer algebra system, used by autocalculate for symbolic work
//! (solve, simplify, differentiate, integrate). See ADR-0005.
//!
//! Each [`Cas`] owns one Giac context. Giac is not thread-safe, so `Cas` is
//! `!Sync`; create one per worker thread.

#[cxx::bridge(namespace = "pennoa")]
mod ffi {
    unsafe extern "C++" {
        include!("shim.h");

        type Session;

        fn new_session() -> UniquePtr<Session>;
        fn eval(session: Pin<&mut Session>, expr: &str) -> Result<String>;
    }
}

/// Error returned by Giac for invalid input or failed evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasError(pub String);

impl std::fmt::Display for CasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "giac: {}", self.0)
    }
}

impl std::error::Error for CasError {}

/// One Giac session.
pub struct Cas {
    session: cxx::UniquePtr<ffi::Session>,
    _not_sync: std::marker::PhantomData<std::cell::Cell<()>>,
}

impl Default for Cas {
    fn default() -> Self {
        Self::new()
    }
}

impl Cas {
    pub fn new() -> Self {
        Self {
            session: ffi::new_session(),
            _not_sync: std::marker::PhantomData,
        }
    }

    /// Evaluate a Giac expression and return its printed result.
    pub fn eval(&mut self, expr: &str) -> Result<String, CasError> {
        if expr.trim().is_empty() {
            return Err(CasError("empty expression".into()));
        }
        ffi::eval(self.session.pin_mut(), expr).map_err(|e| CasError(e.what().to_owned()))
    }

    /// Solve `equation` for `var`, e.g. `solve("x^2-5x+6=0", "x")`.
    pub fn solve(&mut self, equation: &str, var: &str) -> Result<String, CasError> {
        self.eval(&format!("solve({equation},{var})"))
    }

    pub fn simplify(&mut self, expr: &str) -> Result<String, CasError> {
        self.eval(&format!("simplify({expr})"))
    }

    pub fn diff(&mut self, expr: &str, var: &str) -> Result<String, CasError> {
        self.eval(&format!("simplify(diff({expr},{var}))"))
    }

    /// Antiderivative, simplified (Giac returns e.g. `2*x^2/2` otherwise).
    pub fn integrate(&mut self, expr: &str, var: &str) -> Result<String, CasError> {
        self.eval(&format!("simplify(integrate({expr},{var}))"))
    }

    /// Numeric value with `digits` significant digits.
    pub fn evalf(&mut self, expr: &str, digits: u32) -> Result<String, CasError> {
        self.eval(&format!("evalf({expr},{digits})"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(s: String) -> String {
        s.replace(' ', "")
    }

    #[test]
    fn arithmetic_is_exact() {
        let mut cas = Cas::new();
        assert_eq!(norm(cas.eval("12+3*5").unwrap()), "27");
        assert_eq!(norm(cas.eval("1/3+1/6").unwrap()), "1/2");
    }

    #[test]
    fn kinematics_example_from_the_prototype() {
        let mut cas = Cas::new();
        // s = u t + 1/2 a t^2 with u = 12, a = 3, t = 5  ->  97.5 m
        assert_eq!(norm(cas.eval("12*5+1/2*3*5^2").unwrap()), "195/2");
        assert_eq!(norm(cas.evalf("12*5+1/2*3*5^2", 10).unwrap()), "97.5");
    }

    #[test]
    fn solve_quadratic() {
        let mut cas = Cas::new();
        assert_eq!(norm(cas.solve("x^2-5*x+6=0", "x").unwrap()), "list[2,3]");
    }

    #[test]
    fn calculus() {
        let mut cas = Cas::new();
        assert_eq!(norm(cas.diff("x^3+sin(x)", "x").unwrap()), "3*x^2+cos(x)");
        assert_eq!(norm(cas.integrate("2*x", "x").unwrap()), "x^2");
    }

    #[test]
    fn simplify_trig_identity() {
        let mut cas = Cas::new();
        assert_eq!(norm(cas.simplify("sin(x)^2+cos(x)^2").unwrap()), "1");
    }

    #[test]
    fn bad_input_is_an_error_not_a_crash() {
        let mut cas = Cas::new();
        assert!(cas.eval("").is_err());
        assert!(cas.eval("1/(").is_err());
        // The session stays usable after an error.
        assert_eq!(norm(cas.eval("2+2").unwrap()), "4");
    }
}
