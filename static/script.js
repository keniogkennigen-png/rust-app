let socket = null;
let currentSessionKey = null;

// 1. LOGIN / REGISTER FUNCTION
async function authenticate(type) {
    const usernameEl = document.getElementById('username');
    const passwordEl = document.getElementById('password');

    if (!usernameEl || !passwordEl) return;

    const username = usernameEl.value;
    const password = passwordEl.value;

    try {
        const response = await fetch(`/${type}`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });

        const data = await response.json();

        if (response.ok && data.session_key) {
            // Success: Store credentials and switch screens
            currentSessionKey = data.session_key;
            localStorage.setItem('session_key', data.session_key);
            localStorage.setItem('username', data.username);
            
            document.getElementById('auth-screen').style.display = 'none';
            document.getElementById('chat-screen').style.display = 'block';

            // Now that we HAVE a key, establish the uplink
            connectWebSocket(data.session_key);
            
            if (typeof fetchContacts === "function") fetchContacts();
        } else {
            alert(data.message || "Uplink denied by command.");
        }
    } catch (err) {
        console.error("Auth System Error:", err);
        alert("System Error: Communications array offline.");
    }
}

// 2. WEBSOCKET CONNECTION
function connectWebSocket(sessionKey) {
    // STOP the loop if the key is missing
    if (!sessionKey || sessionKey === "undefined") {
        console.warn("Uplink aborted: No valid Session Key.");
        return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const wsUrl = `${protocol}//${host}/chat/${sessionKey}`;

    console.log("Attempting uplink to:", wsUrl);
    
    // Close existing socket if it exists
    if (socket) socket.close();
    
    socket = new WebSocket(wsUrl);

    socket.onopen = () => {
        console.log("Uplink Established.");
        updateStatus("ONLINE");
    };

    socket.onmessage = (event) => {
        try {
            const msg = JSON.parse(event.data);
            displayMessage(msg);
        } catch (e) {
            console.error("Failed to parse data packet:", event.data);
        }
    };

    socket.onclose = (event) => {
        console.log("Uplink lost. Code:", event.code);
        updateStatus("OFFLINE");
        
        // Reconnect after 5 seconds ONLY if we are logged in
        if (currentSessionKey) {
            setTimeout(() => connectWebSocket(currentSessionKey), 5000);
        }
    };

    socket.onerror = (error) => {
        console.error("Transmission Protocol Error:", error);
    };
}

// 3. MESSAGE TRANSMISSION
function sendMessage() {
    const input = document.getElementById('message-input');
    const target = document.getElementById('target-user-id');

    if (socket && socket.readyState === WebSocket.OPEN && input.value) {
        const payload = {
            type: "chatMessage",
            toUserId: target ? target.value : "all",
            message: input.value
        };
        socket.send(JSON.stringify(payload));
        input.value = '';
    }
}

// 4. UI HELPERS
function displayMessage(msg) {
    const chatBox = document.getElementById('chat-box');
    if (!chatBox) return;

    const div = document.createElement('div');
    div.className = "mb-2 text-sm animate-pulse";
    const time = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    
    div.innerHTML = `<span class="opacity-50">[${time}]</span> <span class="text-cyan-400">${msg.fromUsername}:</span> <span class="text-white">${msg.message}</span>`;
    
    chatBox.appendChild(div);
    chatBox.scrollTop = chatBox.scrollHeight;
}

function updateStatus(status) {
    const el = document.getElementById('connection-status');
    if (el) {
        el.innerText = `STATUS: ${status}`;
        el.className = status === "ONLINE" ? "text-green-500" : "text-red-500";
    }
}
