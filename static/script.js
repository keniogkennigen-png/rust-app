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
            // Store session data
            currentSessionKey = data.session_key;
            localStorage.setItem('session_key', data.session_key);
            localStorage.setItem('username', data.username);
            
            // Switch UI screens
            document.getElementById('auth-screen').style.display = 'none';
            document.getElementById('chat-screen').style.display = 'block';

            // Establish secure uplink now that we have a valid key
            connectWebSocket(data.session_key);
            
            // If you have a contact list function, trigger it here
            if (typeof fetchContacts === "function") fetchContacts();
            
        } else {
            alert(data.message || "Uplink denied by server.");
        }
    } catch (err) {
        console.error("Auth System Error:", err);
        alert("System Error: Could not reach authentication server.");
    }
}

// 2. WEBSOCKET CONNECTION (The "Transmission" Logic)
function connectWebSocket(sessionKey) {
    // SAFETY GATE: Prevent connection attempts if key is missing or invalid
    if (!sessionKey || sessionKey === "undefined") {
        console.warn("Uplink aborted: Valid Session Key required.");
        return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const wsUrl = `${protocol}//${host}/chat/${sessionKey}`;

    console.log("Attempting uplink to:", wsUrl);
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
            console.error("Transmission Error: Failed to parse incoming data.", e);
        }
    };

    socket.onclose = (event) => {
        console.log("Uplink lost. Code:", event.code);
        updateStatus("OFFLINE - Reconnecting...");
        
        // Attempt reconnection after 5 seconds if we still have a key
        if (currentSessionKey) {
            setTimeout(() => connectWebSocket(currentSessionKey), 5000);
        }
    };

    socket.onerror = (error) => {
        console.error("WebSocket Protocol Error:", error);
    };
}

// 3. SEND MESSAGE FUNCTION
function sendMessage() {
    const input = document.getElementById('message-input');
    const targetUser = document.getElementById('target-user-id'); // Ensure this ID exists in HTML

    if (socket && socket.readyState === WebSocket.OPEN && input.value) {
        const payload = {
            type: "chatMessage",
            toUserId: targetUser ? targetUser.value : "all", // Fallback if no target selected
            message: input.value
        };
        socket.send(JSON.stringify(payload));
        input.value = '';
    } else {
        console.warn("Cannot send: Socket not connected.");
    }
}

// 4. UI UPDATE HELPERS
function displayMessage(msg) {
    const chatBox = document.getElementById('chat-box');
    if (!chatBox) return;

    const msgElement = document.createElement('div');
    msgElement.className = "message-entry mb-2 text-sm";
    
    // Formatting for the Sci-Fi theme
    const timestamp = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    msgElement.innerHTML = `<span class="text-gray-500">[${timestamp}]</span> <span class="text-cyan-400">${msg.fromUsername}:</span> <span class="text-white">${msg.message}</span>`;
    
    chatBox.appendChild(msgElement);
    chatBox.scrollTop = chatBox.scrollHeight; // Auto-scroll to bottom
}

function updateStatus(status) {
    const statusEl = document.getElementById('connection-status');
    if (statusEl) {
        statusEl.innerText = `STATUS: ${status}`;
        // Optional: Change color based on status
        statusEl.style.color = status === "ONLINE" ? "#4ade80" : "#ef4444";
    }
}

// 5. AUTO-LOGIN CHECK (Optional)
window.onload = () => {
    const savedKey = localStorage.getItem('session_key');
    if (savedKey) {
        currentSessionKey = savedKey;
        // Optionally auto-connect here if you want to skip login screen
        // document.getElementById('auth-screen').style.display = 'none';
        // document.getElementById('chat-screen').style.display = 'block';
        // connectWebSocket(savedKey);
    }
};
