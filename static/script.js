let socket = null;
let currentSessionKey = null;

// 1. LOGIN / REGISTER FUNCTION
async function authenticate(type) {
    const username = document.getElementById('username').value;
    const password = document.getElementById('password').value;

    try {
        const response = await fetch(`/${type}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });

        const data = await response.json();

        if (response.ok && data.session_key) {
            // 1. Store the key
            localStorage.setItem('session_key', data.session_key);
            localStorage.setItem('username', data.username);
            
            // 2. Switch screens
            document.getElementById('auth-screen').style.display = 'none';
            document.getElementById('chat-screen').style.display = 'block';

            // 3. ONLY NOW connect to the server
            connectWebSocket(data.session_key);
            fetchContacts(); // Call this now that we are authenticated
        } else {
            alert(data.message || "Uplink denied.");
        }
    } catch (err) {
        console.error("Auth System Error:", err);
    }
}

// 2. WEBSOCKET CONNECTION (The "Transmission" Logic)
function connectWebSocket(sessionKey) {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    
    // Connect to the /chat/:sessionKey route we defined in main.rs
    socket = new WebSocket(`${protocol}//${host}/chat/${sessionKey}`);

    socket.onopen = () => {
        console.log("Uplink Established.");
        updateStatus("ONLINE");
    };

    socket.onmessage = (event) => {
        const msg = JSON.parse(event.data);
        displayMessage(msg);
    };

    socket.onclose = () => {
        updateStatus("OFFLINE - Reconnecting...");
        setTimeout(() => connectWebSocket(sessionKey), 3000);
    };

    socket.onerror = (error) => {
        console.error("Transmission Error:", error);
    };
}

// 3. SEND MESSAGE FUNCTION
function sendMessage() {
    const input = document.getElementById('message-input');
    const targetUser = document.getElementById('target-user-id').value;

    if (socket && input.value) {
        const payload = {
            type: "chatMessage",
            toUserId: targetUser,
            message: input.value
        };
        socket.send(JSON.stringify(payload));
        input.value = '';
    }
}

function displayMessage(msg) {
    const chatBox = document.getElementById('chat-box');
    const msgElement = document.createElement('div');
    msgElement.innerText = `[${msg.fromUsername}]: ${msg.message}`;
    chatBox.appendChild(msgElement);
}

function updateStatus(status) {
    document.getElementById('connection-status').innerText = `STATUS: ${status}`;
}
