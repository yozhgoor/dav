# Dav

Simple CardDav server implemented in Rust.

## Usage

Run the application using:
```
cargo run
```

### Health check

You can check the status of the server using:
```
curl -i http://127.0.0.1:3000/health
```

You should receive a `200 OK`.

### Create a contact

You can create a new contact using the following command:
```
curl -X POST http://127.0.0.1:3000/contacts \
    -H "Content-Type: application/json" \
    -d '{"id":"123", "name":"John Doe", "email":john@example.com", "phone":"123456789"}'
```

### Delete a contact

You can delete a contact using the following:
```
curl -X DELETE http://127.0.0.1:3000/contacts/<contact_id>
```

### Retrieve a contact using his id

To retrieve a contact, you can use the following:
```
curl http://127.0.0.1:3000/contacts/<contact_id>
```

The response should look like this:
```
BEGIN:VCARD
VERSION:4.0
FN:John Doe
EMAIL:john@example.com
TEL:123456789
END:VCARD
```

### List all the contacts

To get the contact list, you can use the following:
```
curl http://127.0.0.1:3000/contacts
```

The response should be a JSON array with all the available contacts like this:
```json
[
  {
    "id": "123",
    "name": "John Doe",
    "email": "john@example.com",
    "phone": "123-456-789",
  },
]
```

## Local storage

The contacts are stored locally using the following:
| Platform | Value | Example |
| -------- | ----- | ------- |
| Linux | `$XDG_DATA_HOME/dav` or `$HOME/.local/share/dav` | `/home/user/.local/share/dav` |
| macOS | `$HOME/Library/Application Support/dav` | `/Users/Alice/Library/Application Support/dav` |
| Windows | `{FOLDERID_RoamingAppData}\dav\data` | `C:\Users\User\AppData\Roaming\dav\data` |
