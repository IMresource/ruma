//! Endpoints for room creation.

use bodyparser;
use iron::{Chain, Handler, IronError, IronResult, Plugin, Request, Response};
use iron::status::Status;

use config::Config;
use db::DB;
use error::APIError;
use middleware::{AccessTokenAuth, JsonRequest};
use modifier::SerializableResponse;
use room::{CreationOptions, NewRoom, Room};
use user::User;

#[derive(Clone, Debug, Deserialize)]
struct CreateRoomRequest {
    pub room_alias_name: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateRoomResponse {
    room_id: String,
}

/// The /createRoom endpoint.
pub struct CreateRoom;

impl CreateRoomRequest {
    pub fn validate(self) -> Result<Self, IronError> {
        if let Some(ref visibility) = self.visibility {
            if visibility != "public" && visibility != "private" {
                let error = APIError::bad_json();

                return Err(IronError::new(error.clone(), error));
            }
        }

        Ok(self)
    }
}

impl CreateRoom {
    /// Create a `CreateRoom` with all necessary middleware.
    pub fn chain() -> Chain {
        let mut chain = Chain::new(CreateRoom);

        chain.link_before(JsonRequest);
        chain.link_before(AccessTokenAuth);

        chain
    }
}

impl Handler for CreateRoom {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {
        let user = request.extensions.get::<User>()
            .expect("AccessTokenAuth should ensure a user").clone();
        let create_room_request = match request.get::<bodyparser::Struct<CreateRoomRequest>>() {
            Ok(Some(create_room_request)) => create_room_request.validate()?,
            Ok(None) | Err(_) => {
                let error = APIError::bad_json();

                return Err(IronError::new(error.clone(), error));
            }
        };

        let connection = DB::from_request(request)?;
        let config = Config::from_request(request)?;

        let new_room = NewRoom {
            id: Room::generate_room_id(),
            user_id: user.id,
            public: create_room_request.visibility.map_or(false, |v| v == "public"),
        };

        let creation_options = CreationOptions {
            alias: create_room_request.room_alias_name,
        };

        let room = Room::create(&connection, &new_room, &config.domain, &creation_options)?;

        let response = CreateRoomResponse {
            room_id: room.id,
        };

        Ok(Response::with((Status::Ok, SerializableResponse(response))))
    }
}

#[cfg(test)]
mod tests {
    use test::Test;

    #[test]
    fn no_parameters() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?token={}", access_token);

        let response = test.post(&create_room_path, "{}");

        assert!(response.json().find("room_id").unwrap().as_string().is_some());
    }

    #[test]
    fn with_room_alias() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?token={}", access_token);

        let response = test.post(&create_room_path, r#"{"room_alias_name": "my_room"}"#);

        assert!(response.json().find("room_id").unwrap().as_string().is_some());
    }

    #[test]
    fn with_public_visibility() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?token={}", access_token);

        let response = test.post(&create_room_path, r#"{"visibility": "public"}"#);

        assert!(response.json().find("room_id").unwrap().as_string().is_some());
    }

    #[test]
    fn with_private_visibility() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?token={}", access_token);

        let response = test.post(&create_room_path, r#"{"visibility": "private"}"#);

        assert!(response.json().find("room_id").unwrap().as_string().is_some());
    }

    #[test]
    fn with_invalid_visibility() {
        let test = Test::new();
        let access_token = test.create_access_token();

        let create_room_path = format!("/_matrix/client/r0/createRoom?token={}", access_token);

        let response = test.post(&create_room_path, r#"{"visibility": "bogus"}"#);

        assert_eq!(
            response.json().find("errcode").unwrap().as_string().unwrap(),
            "M_BAD_JSON"
        );
    }
}
