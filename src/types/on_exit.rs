use crate::{Address, LazyLoadBlob, Request};

#[derive(Clone, Debug)]
pub enum OnExit {
    None,
    Restart,
    Requests(Vec<Request>),
}

impl OnExit {
    /// Call the kernel to get the current set OnExit behavior
    pub fn get() -> Self {
        match crate::kinode::process::standard::get_on_exit() {
            crate::kinode::process::standard::OnExit::None => OnExit::None,
            crate::kinode::process::standard::OnExit::Restart => OnExit::Restart,
            crate::kinode::process::standard::OnExit::Requests(reqs) => {
                let mut requests: Vec<Request> = Vec::with_capacity(reqs.len());
                for req in reqs {
                    requests.push(Request {
                        target: Some(req.0),
                        inherit: req.1.inherit,
                        timeout: req.1.expects_response,
                        body: Some(req.1.body),
                        metadata: req.1.metadata,
                        blob: req.2,
                        context: None,
                        capabilities: req.1.capabilities,
                    });
                }
                OnExit::Requests(requests)
            }
        }
    }
    /// Check if this OnExit is None
    pub fn is_none(&self) -> bool {
        match self {
            OnExit::None => true,
            OnExit::Restart => false,
            OnExit::Requests(_) => false,
        }
    }
    /// Check if this OnExit is Restart
    pub fn is_restart(&self) -> bool {
        match self {
            OnExit::None => false,
            OnExit::Restart => true,
            OnExit::Requests(_) => false,
        }
    }
    /// Check if this OnExit is Requests
    pub fn is_requests(&self) -> bool {
        match self {
            OnExit::None => false,
            OnExit::Restart => false,
            OnExit::Requests(_) => true,
        }
    }
    /// Get the Requests variant of this OnExit, if it is one
    pub fn get_requests(&self) -> Option<&[Request]> {
        match self {
            OnExit::None => None,
            OnExit::Restart => None,
            OnExit::Requests(reqs) => Some(reqs),
        }
    }
    /// Add a request to this OnExit if it is Requests, fail otherwise
    pub fn add_request(&mut self, new: Request) -> anyhow::Result<()> {
        match self {
            OnExit::None => Err(anyhow::anyhow!("cannot add request to None")),
            OnExit::Restart => Err(anyhow::anyhow!("cannot add request to Restart")),
            OnExit::Requests(ref mut reqs) => {
                reqs.push(new);
                Ok(())
            }
        }
    }
    /// Set the OnExit behavior for this process
    pub fn set(self) -> anyhow::Result<()> {
        crate::kinode::process::standard::set_on_exit(&self._to_standard()?);
        Ok(())
    }
    /// Convert this OnExit to the kernel's OnExit type
    pub fn _to_standard(self) -> anyhow::Result<crate::kinode::process::standard::OnExit> {
        match self {
            OnExit::None => Ok(crate::kinode::process::standard::OnExit::None),
            OnExit::Restart => Ok(crate::kinode::process::standard::OnExit::Restart),
            OnExit::Requests(reqs) => {
                let mut kernel_reqs: Vec<(
                    Address,
                    crate::kinode::process::standard::Request,
                    Option<LazyLoadBlob>,
                )> = Vec::with_capacity(reqs.len());
                for req in reqs {
                    kernel_reqs.push((
                        req.target
                            .ok_or(anyhow::anyhow!("request without target given"))?,
                        crate::kinode::process::standard::Request {
                            inherit: req.inherit,
                            expects_response: None,
                            body: req
                                .body
                                .ok_or(anyhow::anyhow!("request without body given"))?,
                            metadata: req.metadata,
                            capabilities: req.capabilities,
                        },
                        req.blob,
                    ));
                }
                Ok(crate::kinode::process::standard::OnExit::Requests(
                    kernel_reqs,
                ))
            }
        }
    }
}
