# HTTP Message transfer

This utility acts as a reverse proxy, meaning it needs to transfer HTTP request from the requester to the QUIC server, then to the QUIC client and then -> local HTTP server, take the HTTP response and do the same process in reverse.

There are couple of ways we can handle that.

1. Parse the HTTP response in the QUIC client and use the HTTP client to send the request to the local server.
2. Use TCP to send the raw HTTP to the local server, since the HTTP.

While parsing HTTP is much more reliable and provides features that often implemented within HTTP clients, it'll make the processing slower and much harder to implement.

Sending raw HTTP over the TCP, however, will be less reliable, but easier to implement.

## MVP

For the MVP scope, it makes sense to go with the easier solution and build then on top of it the fully functional HTTP client parsing/support.
