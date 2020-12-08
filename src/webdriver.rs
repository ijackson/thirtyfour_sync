use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use log::error;
use serde::Serialize;
use serde_json::Value;

use crate::http::connection_sync::WebDriverHttpClientSync;
use crate::http::reqwest_sync::ReqwestDriverSync;
use crate::webdrivercommands::{start_session, WebDriverCommands};
use crate::WebDriverSession;
use crate::{common::command::Command, error::WebDriverResult, DesiredCapabilities};
use std::time::Duration;

/// The WebDriver struct represents a browser session.
///
/// For full documentation of all WebDriver methods,
/// see the [WebDriverCommands](trait.WebDriverCommands.html) trait.
pub type WebDriver = GenericWebDriver<ReqwestDriverSync>;

/// **NOTE:** For WebDriver method documentation,
/// see the [WebDriverCommands](trait.WebDriverCommands.html) trait.
///
/// The `thirtyfour` crate uses a generic struct that implements the
/// `WebDriverCommands` trait. The generic struct is then implemented for
/// a specific HTTP client.
///
/// This `GenericWebDriver` struct encapsulates a synchronous Selenium WebDriver browser
/// session. For the async driver, see [GenericWebDriver](../struct.GenericWebDriver.html).
///
/// # Example:
/// ```rust
/// use thirtyfour_sync::prelude::*;
///
/// fn main() -> WebDriverResult<()> {
///     let caps = DesiredCapabilities::chrome();
///     let driver = WebDriver::new("http://localhost:4444/wd/hub", &caps)?;
///     driver.get("http://webappdemo")?;
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct GenericWebDriver<T: WebDriverHttpClientSync> {
    pub session: WebDriverSession,
    capabilities: Value,
    quit_on_drop: bool,
    phantom: PhantomData<T>,
}

impl<T: 'static> GenericWebDriver<T>
where
    T: WebDriverHttpClientSync,
{
    /// The GenericWebDriver struct is not intended to be created directly.
    ///
    /// Instead you would use the WebDriver struct, which wires up the
    /// GenericWebDriver with a HTTP client for making requests to the
    /// WebDriver server.
    ///
    /// Create a new WebDriver as follows:
    ///
    /// # Example
    /// ```rust
    /// # use thirtyfour_sync::prelude::*;
    /// #
    /// let caps = DesiredCapabilities::chrome();
    /// let driver = WebDriver::new("http://localhost:4444/wd/hub", &caps)
    ///     .expect("Error starting browser");
    /// ```
    pub fn new<C>(remote_server_addr: &str, capabilities: C) -> WebDriverResult<Self>
    where
        C: Serialize,
    {
        let conn = Arc::new(Mutex::new(T::create(remote_server_addr)?));
        let (session_id, session_capabilities) = start_session(conn.clone(), capabilities)?;
        let driver = GenericWebDriver {
            session: WebDriverSession::new(session_id, conn),
            capabilities: session_capabilities,
            quit_on_drop: true,
            phantom: PhantomData,
        };

        Ok(driver)
    }

    /// Return a clone of the capabilities as returned by Selenium.
    pub fn capabilities(&self) -> DesiredCapabilities {
        DesiredCapabilities::new(self.capabilities.clone())
    }

    /// End the webdriver session.
    pub fn quit(mut self) -> WebDriverResult<()> {
        self.cmd(Command::DeleteSession)?;
        self.quit_on_drop = false;
        Ok(())
    }

    /// Set the request timeout for the HTTP client.
    ///
    /// # Example
    /// ```rust
    /// # use thirtyfour_sync::prelude::*;
    /// # use std::time::Duration;
    /// #
    /// # fn main() -> WebDriverResult<()> {
    /// let caps = DesiredCapabilities::chrome();
    /// let mut driver = WebDriver::new("http://localhost:4444/wd/hub", &caps)?;
    /// driver.set_request_timeout(Duration::from_secs(180))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_request_timeout(&mut self, timeout: Duration) -> WebDriverResult<()> {
        self.session.set_request_timeout(timeout)
    }
}

impl<T> WebDriverCommands for GenericWebDriver<T>
where
    T: WebDriverHttpClientSync,
{
    fn session(&self) -> &WebDriverSession {
        &self.session
    }
}

impl<T> Drop for GenericWebDriver<T>
where
    T: WebDriverHttpClientSync,
{
    /// Close the current session when the WebDriver struct goes out of scope.
    fn drop(&mut self) {
        if self.quit_on_drop && !(self.session.session_id()).is_empty() {
            if let Err(e) = self.cmd(Command::DeleteSession) {
                error!("Failed to close session: {:?}", e);
            }
        }
    }
}
