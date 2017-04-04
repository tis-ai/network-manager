extern crate dbus;

use std;
use general::*;
use device::DeviceType;
use manager::NetworkManager;

/// Get a list of Network Manager connections sorted by path.
///
/// # Examples
///
/// ```no_run
/// use network_manager::connection;
/// let manager = network_manager::manager::NetworkManager::new();
/// let connections = connection::list(&manager).unwrap();
/// println!("{:?}", connections);
/// ```
pub fn list(manager: &NetworkManager) -> Result<Vec<Connection>, String> {
    let paths = try!(manager.list_connections());

    let mut connections = Vec::new();

    for path in &paths {
        connections.push(try!(get_connection(&manager, path)))
    }

    connections.sort();

    Ok(connections)
}

#[test]
fn test_list_function() {
    let manager = NetworkManager::new();

    let connections = list(&manager).unwrap();
    assert!(connections.len() > 0);

    for (index, val) in connections.iter().enumerate() {
        assert_ne!(Connection { ..Default::default() }, val.clone());
        assert_eq!(index as i32, i32::from(val));
    }
}

/// Creates a Network Manager connection.
///
/// # Examples
///
/// ```
/// let connection = network_manager::connection::create(
///     "resin_io",
///     network_manager::device::DeviceType::WiFi,
///     network_manager::general::Security::WPA2,
///     "super_secret_passphase"
///     ).unwrap();
/// println!("{:?}", connection);
/// ```
pub fn create(s: &str, dt: DeviceType, sc: Security, p: &str) -> Result<Connection, String> {
    // Create a connection
    // Get the connection
    // Return the connection

    let settings = ConnectionSettings {
        id: "resin_io".to_string(),
        uuid: "3c8e6e8b-b895-4b07-97a5-bbc192c3b436".to_string(),
        ssid: "resin_io".to_string(),
    };

    let connection1 = Connection {
        path: "/org/freedesktop/NetworkManager/ActiveConnection/187".to_string(),
        active_path: "test".to_string(),
        settings: settings,
        state: ConnectionState::Deactivated, /* device: "wlp4s0".to_string(),
                                              * interface: DeviceType::WiFi,
                                              * security: Security::WPA2,
                                              * state: ConnectionState::Activated, */
    };

    Ok(connection1)
}

/// Deletes a Network Manager connection.
///
/// # Examples
///
/// ```
/// let manager = network_manager::manager::NetworkManager::new();
/// let mut connections = network_manager::connection::list(&manager).unwrap();
/// network_manager::connection::delete(connections.pop().unwrap()).unwrap();
/// ```
pub fn delete(connection: Connection) -> Result<(), String> {
    let message = dbus_message!(NM_SERVICE_MANAGER,
                                connection.path,
                                NM_CONNECTION_INTERFACE,
                                "Delete");
    dbus_connect!(message);

    Ok(())
}

/// Enables a Network Manager connection.
///
/// # Examples
///
/// ```no_run
/// use network_manager::connection;
/// let manager = network_manager::manager::NetworkManager::new();
/// let connections = connection::list(&manager).unwrap();
/// let mut connection = connections[0].clone();
/// connection::enable(&manager, &mut connection, 10).unwrap();
/// println!("{:?}", connection.state);
/// ```
pub fn enable(manager: &NetworkManager,
              connection: &mut Connection,
              time_out: i32)
              -> Result<(), String> {
    match connection.state {
        ConnectionState::Activated => Ok(()),
        ConnectionState::Activating => {
            wait(manager, connection, time_out, ConnectionState::Activated)
        }
        ConnectionState::Unknown => Err("Unable to get connection state".to_string()),
        _ => {
            let mut message = dbus_message!(NM_SERVICE_MANAGER,
                                            NM_SERVICE_PATH,
                                            NM_SERVICE_INTERFACE,
                                            "ActivateConnection");
            message.append_items(&[dbus::MessageItem::ObjectPath(connection
                                                                     .path
                                                                     .to_string()
                                                                     .into()),
                                   dbus::MessageItem::ObjectPath("/".into()),
                                   dbus::MessageItem::ObjectPath("/".into())]);
            dbus_connect!(message);

            wait(manager, connection, time_out, ConnectionState::Activated)
        }
    }
}

/// Disables a Network Manager connection.
///
/// # Examples
///
/// ```no_run
/// use network_manager::connection;
/// let manager = network_manager::manager::NetworkManager::new();
/// let connections = connection::list(&manager).unwrap();
/// let mut connection = connections[0].clone();
/// connection::disable(&manager, &mut connection, 10).unwrap();
/// println!("{:?}", connection.state);
/// ```
pub fn disable(manager: &NetworkManager,
               connection: &mut Connection,
               time_out: i32)
               -> Result<(), String> {
    match connection.state {
        ConnectionState::Deactivated => Ok(()),
        ConnectionState::Deactivating => {
            wait(manager, connection, time_out, ConnectionState::Deactivated)
        }
        ConnectionState::Unknown => Err("Unable to get connection state".to_string()),
        _ => {
            let mut message = dbus_message!(NM_SERVICE_MANAGER,
                                            NM_SERVICE_PATH,
                                            NM_SERVICE_INTERFACE,
                                            "DeactivateConnection");
            message.append_items(&[dbus::MessageItem::ObjectPath(connection
                                                                     .active_path
                                                                     .to_string()
                                                                     .into())]);
            dbus_connect!(message);

            wait(manager, connection, time_out, ConnectionState::Deactivated)
        }
    }
}

#[test]
fn test_enable_disable_functions() {
    let manager = NetworkManager::new();

    let connections = list(&manager).unwrap();
    let mut connection;


    // set enviorment variable $TEST_WIFI_SSID with the wifi's SSID that you want to test
    // e.g.  export TEST_WIFI_SSID="Resin.io Wifi"
    let wifiEnvVar = "TEST_WIFI_SSID";
    match env::var(wifiEnvVar) {
        Ok(ssid) => {
            connection = connections
                .iter()
                .filter(|c| c.settings.ssid == ssid)
                .nth(0)
                .unwrap()
                .clone()
        }
        Err(e) => {
            panic!("couldn't retrieve enviorment variable {}: {}",
                   wifiEnvVar,
                   e)
        }
    };

    if connection.state == ConnectionState::Activated {
        disable(&manager, &mut connection, 10).unwrap();
        assert_eq!(ConnectionState::Deactivated, connection.state);

        enable(&manager, &mut connection, 10).unwrap();
        assert_eq!(ConnectionState::Activated, connection.state);
    } else {
        enable(&manager, &mut connection, 10).unwrap();
        assert_eq!(ConnectionState::Activated, connection.state);

        disable(&manager, &mut connection, 10).unwrap();
        assert_eq!(ConnectionState::Deactivated, connection.state);
    }
}

fn get_connection(manager: &NetworkManager, path: &String) -> Result<Connection, String> {
    let mut connection = Connection::default();

    connection.path = path.clone();

    connection.settings = try!(manager.get_connection_settings(path));

    try!(update_state(manager, &mut connection));

    Ok(connection)
}

fn update_state(manager: &NetworkManager, connection: &mut Connection) -> Result<(), String> {
    let active_paths = try!(manager.get_active_connections());

    let mut settings_paths = Vec::new();

    for active_path in &active_paths {
        if let Some(settings_path) = manager.get_active_connection_path(&active_path) {
            settings_paths.push(settings_path)
        }
    }

    // TODO: Consider using Option<String> instead for deactivated connections
    connection.active_path = "".to_string();
    connection.state = ConnectionState::Deactivated;

    for (active_path, settings_path) in active_paths.iter().zip(settings_paths.iter()) {
        if connection.path == *settings_path {

            connection.active_path = active_path.clone();
            connection.state = try!(manager.get_connection_state(&active_path));

            break;
        }
    }

    Ok(())
}

fn wait(manager: &NetworkManager,
        connection: &mut Connection,
        time_out: i32,
        target_state: ConnectionState)
        -> Result<(), String> {
    if time_out == 0 {
        return Ok(());
    }

    let mut total_time = 0;

    while total_time < time_out {
        try!(update_state(manager, connection));

        if connection.state == target_state {
            return Ok(());
        }

        std::thread::sleep(std::time::Duration::from_secs(1));

        total_time += 1;
    }

    Err("service timed out".to_string())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Connection {
    pub path: String,
    pub active_path: String,
    pub settings: ConnectionSettings,
    pub state: ConnectionState, /* device: String,
                                 * device_type: DeviceType,
                                 * security: Security, */
}

impl Default for Connection {
    fn default() -> Connection {
        Connection {
            path: "".to_string(),
            active_path: "".to_string(),
            settings: ConnectionSettings::default(),
            state: ConnectionState::Unknown,
        }
    }
}

impl Ord for Connection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        i32::from(self).cmp(&i32::from(other))
    }
}

impl PartialOrd for Connection {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> From<&'a Connection> for i32 {
    fn from(val: &Connection) -> i32 {
        val.clone()
            .path
            .rsplit('/')
            .nth(0)
            .unwrap()
            .parse::<i32>()
            .unwrap()
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct ConnectionSettings {
    pub id: String,
    pub uuid: String,
    pub ssid: String,
}
