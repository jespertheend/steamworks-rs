use super::*;
use crate::networking_types::NetworkingIdentity;
#[cfg(test)]
use serial_test::serial;

/// Access to the steam user interface
pub struct User {
    pub(crate) user: *mut sys::ISteamUser,
    pub(crate) _inner: Arc<Inner>,
}

impl User {
    /// Returns the steam id of the current user
    pub fn steam_id(&self) -> SteamId {
        unsafe { SteamId(sys::SteamAPI_ISteamUser_GetSteamID(self.user)) }
    }

    /// Returns the level of the current user
    pub fn level(&self) -> u32 {
        unsafe { sys::SteamAPI_ISteamUser_GetPlayerSteamLevel(self.user) as u32 }
    }

    /// Returns whether the current user's Steam client is connected to the Steam servers.
    pub fn logged_on(&self) -> bool {
        unsafe { sys::SteamAPI_ISteamUser_BLoggedOn(self.user) }
    }

    /// Retrieve an authentication session ticket that can be sent
    /// to an entity that wishes to verify you.
    ///
    /// This ticket should not be reused.
    ///
    /// When creating ticket for use by the web API you should wait
    /// for the `AuthSessionTicketResponse` event before trying to
    /// use the ticket.
    ///
    /// When the multiplayer session terminates you must call
    /// `cancel_authentication_ticket`
    pub fn authentication_session_ticket_with_steam_id(
        &self,
        steam_id: SteamId,
    ) -> (AuthTicket, Vec<u8>) {
        self.authentication_session_ticket(NetworkingIdentity::new_steam_id(steam_id))
    }
    pub fn authentication_session_ticket(
        &self,
        network_identity: NetworkingIdentity,
    ) -> (AuthTicket, Vec<u8>) {
        unsafe {
            let mut ticket = vec![0; 1024];
            let mut ticket_len = 0;
            let auth_ticket = sys::SteamAPI_ISteamUser_GetAuthSessionTicket(
                self.user,
                ticket.as_mut_ptr().cast(),
                1024,
                &mut ticket_len,
                network_identity.as_ptr(),
            );
            ticket.truncate(ticket_len as usize);
            (AuthTicket(auth_ticket), ticket)
        }
    }

    /// Cancels an authentication session ticket received from
    /// `authentication_session_ticket`.
    ///
    /// This should be called when you are no longer playing with
    /// the specified entity.
    pub fn cancel_authentication_ticket(&self, ticket: AuthTicket) {
        unsafe {
            sys::SteamAPI_ISteamUser_CancelAuthTicket(self.user, ticket.0);
        }
    }

    /// Authenticate the ticket from the steam ID to make sure it is
    /// valid and not reused.
    ///
    /// A `ValidateAuthTicketResponse` callback will be fired if
    /// the entity goes offline or cancels the ticket.
    ///
    /// When the multiplayer session terminates you must call
    /// `end_authentication_session`
    pub fn begin_authentication_session(
        &self,
        user: SteamId,
        ticket: &[u8],
    ) -> Result<(), AuthSessionError> {
        unsafe {
            let res = sys::SteamAPI_ISteamUser_BeginAuthSession(
                self.user,
                ticket.as_ptr().cast(),
                ticket.len() as _,
                user.0,
            );
            Err(match res {
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultOK => return Ok(()),
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultInvalidTicket => {
                    AuthSessionError::InvalidTicket
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultDuplicateRequest => {
                    AuthSessionError::DuplicateRequest
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultInvalidVersion => {
                    AuthSessionError::InvalidVersion
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultGameMismatch => {
                    AuthSessionError::GameMismatch
                }
                sys::EBeginAuthSessionResult::k_EBeginAuthSessionResultExpiredTicket => {
                    AuthSessionError::ExpiredTicket
                }
                _ => unreachable!(),
            })
        }
    }

    /// Ends an authentication session that was started with
    /// `begin_authentication_session`.
    ///
    /// This should be called when you are no longer playing with
    /// the specified entity.
    pub fn end_authentication_session(&self, user: SteamId) {
        unsafe {
            sys::SteamAPI_ISteamUser_EndAuthSession(self.user, user.0);
        }
    }

    /// Retrieve an authentication ticket to be sent to the entity that
    /// wishes to authenticate you using the
    /// ISteamUserAuth/AuthenticateUserTicket Web API.
    ///
    /// The calling application must wait for the
    /// `TicketForWebApiResponse` callback generated  
    /// by the API call to access the ticket.
    ///  
    /// It is best practice to use an identity string for
    /// each service that will consume tickets.
    ///   
    /// This API can not be used to create a ticket for
    /// use by the BeginAuthSession/ISteamGameServer::BeginAuthSession.
    /// Use the `authentication_session_ticket` API instead
    pub fn authentication_session_ticket_for_webapi(&self, identity: &str) -> AuthTicket {
        unsafe {
            let c_str = CString::new(identity).unwrap();
            let auth_ticket =
                sys::SteamAPI_ISteamUser_GetAuthTicketForWebApi(self.user, c_str.as_ptr());

            AuthTicket(auth_ticket)
        }
    }

    /// Checks if the user owns a piece of DLC specified by app id.
    ///
    /// This can only be called after authenticating
    /// with the user using `begin_authentication_session`.
    pub fn user_has_license_for_app(&self, user: SteamId, app_id: AppId) -> UserHasLicense {
        unsafe {
            let license_response =
                sys::SteamAPI_ISteamUser_UserHasLicenseForApp(self.user, user.0, app_id.0);

            match license_response {
                sys::EUserHasLicenseForAppResult::k_EUserHasLicenseResultHasLicense => {
                    UserHasLicense::HasLicense
                }
                sys::EUserHasLicenseForAppResult::k_EUserHasLicenseResultDoesNotHaveLicense => {
                    UserHasLicense::DoesNotHaveLicense
                }
                sys::EUserHasLicenseForAppResult::k_EUserHasLicenseResultNoAuth => {
                    UserHasLicense::NoAuth
                }
                _ => unreachable!(),
            }
        }
    }
}

/// Errors from `begin_authentication_session`
#[derive(Debug, Error)]
pub enum AuthSessionError {
    /// The ticket is invalid
    #[error("invalid ticket")]
    InvalidTicket,
    /// A ticket has already been submitted for this steam ID
    #[error("duplicate ticket request")]
    DuplicateRequest,
    /// The ticket is from an incompatible interface version
    #[error("incompatible interface version")]
    InvalidVersion,
    /// The ticket is not for this game
    #[error("incorrect game for ticket")]
    GameMismatch,
    /// The ticket has expired
    #[error("ticket has expired")]
    ExpiredTicket,
}

#[test]
#[serial]
fn test_auth_dll() {
    let client = Client::init().unwrap();
    let user = client.user();

    let _cb = client.register_callback(|v: AuthSessionTicketResponse| {
        println!("Got dll auth response: {:?}", v)
    });
    let _cb = client.register_callback(|v: ValidateAuthTicketResponse| {
        println!("Got validate auth response: {:?}", v)
    });

    let id = user.steam_id();
    let (auth, ticket) = user.authentication_session_ticket_with_steam_id(id);

    println!("{:?}", auth);
    println!("{:?}", ticket);

    println!("{:?}", user.begin_authentication_session(id, &ticket));

    // this might still return NoAuth if the validation has not completed
    let has_space_war = user.user_has_license_for_app(id, AppId(480));
    println!("User has license response: {has_space_war:?}");

    for _ in 0..20 {
        client.run_callbacks();
        ::std::thread::sleep(::std::time::Duration::from_millis(50));
    }

    println!("END");

    user.cancel_authentication_ticket(auth);

    for _ in 0..20 {
        client.run_callbacks();
        ::std::thread::sleep(::std::time::Duration::from_millis(50));
    }

    user.end_authentication_session(id);
}

/// A handle for an authentication ticket that can be used to cancel
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthTicket(pub(crate) sys::HAuthTicket);

/// Called when generating a authentication session ticket.
///
/// This can be used to verify the ticket was created successfully.
#[derive(Clone, Debug)]
pub struct AuthSessionTicketResponse {
    /// The ticket in question
    pub ticket: AuthTicket,
    /// The result of generating the ticket
    pub result: SResult<()>,
}

impl_callback!(cb: GetAuthSessionTicketResponse_t => AuthSessionTicketResponse {
    Self {
        ticket: AuthTicket(cb.m_hAuthTicket),
        result: crate::to_steam_result(cb.m_eResult),
    }
});

#[test]
#[serial]
fn test_auth_webapi() {
    let client = Client::init().unwrap();
    let user = client.user();

    let _cb = client.register_callback(|v: TicketForWebApiResponse| {
        println!("Got webapi auth response: {:?}", v)
    });

    let auth = user.authentication_session_ticket_for_webapi("myIdentity");

    println!("{:?}", auth);

    for _ in 0..20 {
        client.run_callbacks();
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
    }

    println!("END");
}

/// Called when generating a authentication session ticket for web api.
///
/// This can be used to verify the ticket was created successfully.
#[derive(Debug)]
pub struct TicketForWebApiResponse {
    pub ticket_handle: AuthTicket,
    pub result: SResult<()>,
    pub ticket_len: i32,
    pub ticket: Vec<u8>,
}

impl_callback!(cb: GetTicketForWebApiResponse_t => TicketForWebApiResponse {
    Self {
        ticket_handle: AuthTicket(cb.m_hAuthTicket),
        result: crate::to_steam_result(cb.m_eResult),
        ticket_len: cb.m_cubTicket,
        ticket: cb.m_rgubTicket.to_vec(),
    }
});

/// Called when an authentication ticket has been
/// validated.
#[derive(Clone, Debug)]
pub struct ValidateAuthTicketResponse {
    /// The steam id of the entity that provided the ticket
    pub steam_id: SteamId,
    /// The result of the validation
    pub response: Result<(), AuthSessionValidateError>,
    /// The steam id of the owner of the game. Differs from
    /// `steam_id` if the game is borrowed.
    pub owner_steam_id: SteamId,
}

impl_callback!(cb: ValidateAuthTicketResponse_t => ValidateAuthTicketResponse {
    Self {
        steam_id: SteamId(cb.m_SteamID.m_steamid.m_unAll64Bits),
        owner_steam_id: SteamId(cb.m_OwnerSteamID.m_steamid.m_unAll64Bits),
        response: match cb.m_eAuthSessionResponse {
            sys::EAuthSessionResponse::k_EAuthSessionResponseOK => Ok(()),
            sys::EAuthSessionResponse::k_EAuthSessionResponseUserNotConnectedToSteam => {
                Err(AuthSessionValidateError::UserNotConnectedToSteam)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseNoLicenseOrExpired => {
                Err(AuthSessionValidateError::NoLicenseOrExpired)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseVACBanned => {
                Err(AuthSessionValidateError::VACBanned)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseLoggedInElseWhere => {
                Err(AuthSessionValidateError::LoggedInElseWhere)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseVACCheckTimedOut => {
                Err(AuthSessionValidateError::VACCheckTimedOut)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseAuthTicketCanceled => {
                Err(AuthSessionValidateError::AuthTicketCancelled)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseAuthTicketInvalidAlreadyUsed => {
                Err(AuthSessionValidateError::AuthTicketInvalidAlreadyUsed)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponseAuthTicketInvalid => {
                Err(AuthSessionValidateError::AuthTicketInvalid)
            }
            sys::EAuthSessionResponse::k_EAuthSessionResponsePublisherIssuedBan => {
                Err(AuthSessionValidateError::PublisherIssuedBan)
            }
            _ => unreachable!(),
        },
    }
});

// Called when a microtransaction authorization response is received
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MicroTxnAuthorizationResponse {
    pub app_id: AppId,
    pub order_id: u64,
    pub authorized: bool,
}

impl_callback!(cb: MicroTxnAuthorizationResponse_t => MicroTxnAuthorizationResponse {
    Self {
        app_id: cb.m_unAppID.into(),
        order_id: cb.m_ulOrderID.into(),
        authorized: cb.m_bAuthorized == 1,
    }
});

/// Called when a connection to the Steam servers is made.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamServersConnected;

impl_callback!(_cb: SteamServersConnected_t => SteamServersConnected {
    Self
});

/// Called when the connection to the Steam servers is lost.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamServersDisconnected {
    /// The reason we were disconnected from the Steam servers
    pub reason: SteamError,
}

impl_callback!(cb: SteamServersDisconnected_t => SteamServersDisconnected {
    Self {
        reason: cb.m_eResult.into(),
    }
});

/// Called when the connection to the Steam servers fails.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SteamServerConnectFailure {
    /// The reason we failed to connect to the Steam servers
    pub reason: SteamError,
    /// Whether we are still retrying the connection.
    pub still_retrying: bool,
}

impl_callback!(cb: SteamServerConnectFailure_t => SteamServerConnectFailure {
    Self {
        reason: cb.m_eResult.into(),
        still_retrying: cb.m_bStillRetrying,
    }
});

/// Errors from `ValidateAuthTicketResponse`
#[derive(Clone, Debug, Error)]
pub enum AuthSessionValidateError {
    /// The user in question is not connected to steam
    #[error("user not connected to steam")]
    UserNotConnectedToSteam,
    /// The license has expired
    #[error("the license has expired")]
    NoLicenseOrExpired,
    /// The user is VAC banned from the game
    #[error("the user is VAC banned from this game")]
    VACBanned,
    /// The user has logged in elsewhere and the session
    /// has been disconnected
    #[error("the user is logged in elsewhere")]
    LoggedInElseWhere,
    /// VAC has been unable to perform anti-cheat checks on this
    /// user
    #[error("VAC check timed out")]
    VACCheckTimedOut,
    /// The ticket has been cancelled by the issuer
    #[error("the authentication ticket has been cancelled")]
    AuthTicketCancelled,
    /// The ticket has already been used
    #[error("the authentication ticket has already been used")]
    AuthTicketInvalidAlreadyUsed,
    /// The ticket is not from a user instance currently connected
    /// to steam
    #[error("the authentication ticket is invalid")]
    AuthTicketInvalid,
    /// The user is banned from the game (not VAC)
    #[error("the user is banned")]
    PublisherIssuedBan,
}

/// Results from [`User::user_has_license_for_app`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserHasLicense {
    /// The user has a license for the specified app.
    HasLicense,
    /// The user does not have a license for the specified app.
    DoesNotHaveLicense,
    /// The user has not been authenticated.
    NoAuth,
}
