# PortZero CLI Manual Testing Guide

Step-by-step test plan covering every documented CLI command. Run these in order — each section builds on the previous state.

## Prerequisites

```sh
cargo build --release -p portzero-cli
export PATH="$PWD/target/release:$PATH"
```

Verify the binary is accessible:

```sh
portzero --help
```

---

## 1. Daemon Lifecycle

### 1.1 Start the daemon

```sh
portzero start
```

Expected: Daemon starts in the foreground, prints:
```
PortZero proxy starting on http://localhost:1337
Dashboard: http://_portzero.localhost:1337
Control socket: /Users/<you>/.portzero/portzero.sock
```

Leave this running in a terminal. All following tests use a **separate terminal**.

### 1.2 Check daemon status

```sh
portzero status
```

Expected:
```
Daemon is running.
No apps registered.
```

### 1.3 List apps (empty state)

```sh
portzero list
```

Expected:
```
PortZero — http://localhost:1337

No apps running.

Start an app with: portzero <name> <command>
Or start all from config: portzero up
```

---

## 2. Run a Single App

### 2.1 Run with inferred name

From any directory (the directory name becomes the app name):

```sh
portzero python3 -m http.server 0
```

Expected:
- PortZero assigns a port via `$PORT` and spawns the command
- Prints the app URL, e.g. `http://portzero.localhost:1337`
- Traffic is proxied through the `.localhost` URL

Test the proxy works:

```sh
curl http://portzero.localhost:1337
```

Stop with `Ctrl+C`, then verify the app is deregistered:

```sh
portzero list
```

### 2.2 Run with explicit name

```sh
portzero my-server python3 -m http.server 0
```

Expected:
- App registers as `my-server`
- URL is `http://my-server.localhost:1337`

Test:

```sh
curl http://my-server.localhost:1337
```

Stop with `Ctrl+C`.

---

## 3. Run Multiple Apps (`portzero up`)

### 3.1 Create a test config

Create a `portzero.toml` in a test directory:

```toml
[apps.web]
command = "python3 -m http.server $PORT"

[apps.api]
command = "python3 -m http.server $PORT"
```

### 3.2 Start all apps

```sh
portzero up
```

Expected:
- Both apps start with assigned ports
- URLs printed: `http://web.localhost:1337` and `http://api.localhost:1337`
- Logs from both apps are interleaved in the terminal

### 3.3 Verify apps are listed

In another terminal:

```sh
portzero list
```

Expected: Both apps shown with name, URL, port, PID, uptime, and command.

### 3.4 Check status

```sh
portzero status
```

Expected: Shows "Daemon is running." with both apps listed.

### 3.5 View logs

```sh
portzero logs web
```

Expected: Shows buffered log output for the `web` app.

With follow mode:

```sh
portzero logs -f web
```

Expected: Streams new log lines in real-time. Generate traffic to see output:

```sh
curl http://web.localhost:1337
```

Stop with `Ctrl+C`.

### 3.6 Stop all apps

```sh
portzero down
```

Expected: All apps and the daemon stop. Equivalent to `portzero stop`.

---

## 4. Response Mocking

Requires a running daemon and at least one app.

### 4.1 Create a mock

```sh
portzero mock add my-app GET /api/health --status 200 --body '{"status":"ok"}'
```

Expected: Prints mock ID, app, match pattern, status code, and enabled state.

### 4.2 List mocks

```sh
portzero mock list
```

Expected: Table showing the mock rule with ID, app, method, path, status, hits, and enabled.

### 4.3 Test the mock

```sh
curl http://my-app.localhost:1337/api/health
```

Expected: Returns `{"status":"ok"}` with status 200, served by the mock (not the upstream).

### 4.4 Disable a mock

```sh
portzero mock disable <id>
```

Expected: Mock disabled. Requests to `/api/health` now pass through to the upstream.

### 4.5 Enable a mock

```sh
portzero mock enable <id>
```

Expected: Mock re-enabled.

### 4.6 Delete a mock

```sh
portzero mock delete <id>
```

Expected: Mock removed from the list.

---

## 5. Network Simulation (Throttle)

Requires a running daemon and at least one app.

### 5.1 Set latency

```sh
portzero throttle set my-app --latency 500
```

Expected: Prints confirmation with latency settings.

### 5.2 Test latency

```sh
time curl http://my-app.localhost:1337
```

Expected: Request takes ~500ms longer than normal.

### 5.3 Set complex profile

```sh
portzero throttle set my-app --latency 200 --jitter 50 --drop 0.1
```

Expected: 200ms latency +/- 50ms jitter, 10% packet loss.

### 5.4 List active profiles

```sh
portzero throttle list
```

Expected: Table showing the active profile for `my-app`.

### 5.5 Clear simulation

```sh
portzero throttle clear my-app
```

Expected: Network simulation removed.

---

## 6. Traffic Inspection (Web Dashboard)

> Note: sections below were renumbered after adding mock/throttle tests.

### 4.1 Open the dashboard

Start the daemon and an app:

```sh
portzero start &
portzero my-app python3 -m http.server 0
```

Open in a browser: `http://_portzero.localhost:1337`

Expected: Web dashboard loads showing the app and live traffic.

### 4.2 Generate and inspect traffic

```sh
curl http://my-app.localhost:1337
curl http://my-app.localhost:1337/some-path
curl -X POST http://my-app.localhost:1337/api/test -d '{"hello":"world"}'
```

Expected: Each request appears in the dashboard traffic inspector with method, path, status code, and duration.

---

## 7. TLS / Certificate Trust

### 5.1 Trust the CA

```sh
portzero trust
```

Expected: Installs the PortZero CA certificate into the system trust store. May require sudo/password.

### 5.2 Verify HTTPS works

After trusting, HTTPS should work without certificate warnings:

```sh
curl https://my-app.localhost:1337
```

### 5.3 Untrust the CA

```sh
portzero untrust
```

Expected: Removes the CA certificate from the system trust store.

---

## 8. Daemon Stop

### 6.1 Stop the daemon

```sh
portzero stop
```

Expected: Sends SIGTERM to the daemon process, prints confirmation with PID.

### 6.2 Verify it's stopped

```sh
portzero status
```

Expected: `Daemon not running.`

---

## 9. Public Tunnels (requires `--features tunnel`)

> These commands only work if you built with tunnel support:
> `cargo build --release -p portzero-cli --features tunnel`

### 7.1 Login

```sh
portzero login
```

Expected: Prompts for email and password, authenticates with LocalUp.

### 7.2 Check identity

```sh
portzero whoami
```

Expected: Shows the logged-in email and relay URL.

### 7.3 Share an app

Start an app first, then:

```sh
portzero share start my-app
```

Expected: Creates a public tunnel URL, prints it.

### 7.4 List tunnels

```sh
portzero share list
```

Expected: Shows active tunnels with app name, public URL, and transport.

### 7.5 Stop sharing

```sh
portzero share stop my-app
```

Expected: Tunnel stopped.

### 7.6 Logout

```sh
portzero logout
```

Expected: Credentials removed.

---

## Quick Smoke Test (All Core Commands)

If you want a fast end-to-end check, run this sequence:

```sh
# Build
cargo build --release -p portzero-cli

# 1. Start daemon (background)
portzero start &
sleep 2

# 2. Check status
portzero status

# 3. List (empty)
portzero list

# 4. Run an app
portzero test-app python3 -m http.server 0 &
sleep 2

# 5. List (should show test-app)
portzero list

# 6. Status (should show test-app)
portzero status

# 7. Generate traffic
curl -s http://test-app.localhost:1337 > /dev/null

# 8. View logs
portzero logs test-app

# 9. Dashboard
echo "Open http://_portzero.localhost:1337 in browser"

# 10. Cleanup
kill %2        # stop the app
portzero stop  # stop daemon
portzero status  # should say "not running"
```

---

## Known Limitations

Tunnel commands (`share`, `login`, `logout`, `whoami`) require building with `--features tunnel`.
