        // --- Element Selectors ---
        const authSection = document.getElementById('authSection');
        const profileSection = document.getElementById('profileSection');
        const contactsSection = document.getElementById('contactsSection');
        const currentUsernameSpan = document.getElementById('currentUsername');
        const logoutButton = document.getElementById('logoutButton');
        const authUsernameInput = document.getElementById('authUsername');
        const authPasswordInput = document.getElementById('authPassword');
        const registerButton = document.getElementById('registerButton');
        const loginButton = document.getElementById('loginButton');
        const addContactUsernameInput = document.getElementById('addContactUsername');
        const addContactButton = document.getElementById('addContactButton');
        const contactsListDiv = document.getElementById('contactsList');
        const noContactsMessage = document.getElementById('noContactsMessage');
        const messageInput = document.getElementById('messageInput');
        const sendButton = document.getElementById('sendButton');
        const messagesDiv = document.getElementById('messages');
        const chattingWithSpan = document.getElementById('chattingWith');
        const messageBox = document.getElementById('messageBox');
        const typingIndicatorContainer = document.getElementById('typingIndicatorContainer');
        const typingStatusText = document.getElementById('typingStatusText'); // New selector for text within indicator
        const typingBar = document.getElementById('typing-bar');// Get direct reference to the bar
        
        // --- Application State ---
        let currentUser = null; // Stores { user_id, username, session_token }
        let ws = null;
        let currentRecipient = null; // Stores { id, username } of the selected contact
        let allMessages = {}; // Stores messages grouped by contact: { 'contact_user_id': [...] }
        let typingTimeout;

        // --- Utility Functions ---
        function showMessageBox(message, isError = false) {
            messageBox.textContent = message;
            messageBox.className = `message-box ${isError ? 'error' : ''}`;
            messageBox.style.display = 'block';
            setTimeout(() => { messageBox.style.display = 'none'; }, 3000);
        }

        function handleLoginResponse(loginData) {
            const userSession = {
                user_id: loginData.user_id,
                username: loginData.username,
                session_token: loginData.session_key // The key from the server response
            };
            
            currentUser = userSession;
            localStorage.setItem('currentUser', JSON.stringify(userSession));
            
            updateUIForLogin();
            connectWebSocket();
            fetchContacts();
        }
        
        function loadSessionFromStorage() {
            const userString = localStorage.getItem('currentUser');
            if (userString) {
                currentUser = JSON.parse(userString);
                updateUIForLogin();
                connectWebSocket();
                fetchContacts();
            } else {
                updateUIForLogout();
            }
        }
        
        function clearCurrentUser() {
            localStorage.removeItem('currentUser');
            currentUser = null;
            if (ws) {
                ws.close();
                ws = null;
            }
            updateUIForLogout();
        }

        function updateUIForLogin() {
            authSection.classList.add('hidden');
            profileSection.classList.remove('hidden');
            contactsSection.classList.remove('hidden');
            currentUsernameSpan.textContent = currentUser.username;
            chattingWithSpan.textContent = "Select a connection to transmit";
            enableChatInput(false);
            messagesDiv.innerHTML = '<div class="system-message">Connection successful. Select a channel to begin transmission.</div>';
        }

        function updateUIForLogout() {
            authSection.classList.remove('hidden');
            profileSection.classList.add('hidden');
            contactsSection.classList.add('hidden');
            currentUsernameSpan.textContent = '';
            chattingWithSpan.textContent = "Awaiting Connection...";
            enableChatInput(false);
            messagesDiv.innerHTML = '<div class="system-message">Initialize connection protocol to begin.</div>';
            contactsListDiv.innerHTML = '<p class="text-gray-500 text-sm italic" id="noContactsMessage">No connections detected.</p>';
            currentRecipient = null;
        }

        function enableChatInput(enabled) {
            messageInput.disabled = !enabled;
            sendButton.disabled = !enabled;
        }

       function appendMessage(msgData) {
            const { from_username, message, type, from_user_id, timestamp, message_id } = msgData;

            if (type === 'system') {
                const sysMsgDiv = document.createElement('div');
                sysMsgDiv.className = 'system-message';
                sysMsgDiv.textContent = message;
                messagesDiv.appendChild(sysMsgDiv);
            } else {
                const isSelf = from_user_id === currentUser.user_id;
                const messageClass = isSelf ? 'sent' : 'received';

                const messageBubble = document.createElement('div');
                messageBubble.className = `message-bubble ${messageClass}`;
                messageBubble.id = `msg-${message_id}`; 

                if (!isSelf) {
                    const senderSpan = document.createElement('div');
                    senderSpan.className = 'message-sender';
                    senderSpan.textContent = from_username;
                    messageBubble.appendChild(senderSpan);
                }

                const messageText = document.createElement('span');
                messageText.textContent = message;
                messageBubble.appendChild(messageText);

                if (timestamp) {
                    const timeEl = document.createElement('div');
                    timeEl.style.fontSize = '0.7rem';
                    timeEl.style.opacity = 0.6;
                    timeEl.style.marginTop = '0.25rem';
                    timeEl.style.textAlign = 'right';
                    const dt = new Date(timestamp);
                    timeEl.textContent = dt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
                    messageBubble.appendChild(timeEl);
                }

                messagesDiv.appendChild(messageBubble);
            }
            messagesDiv.scrollTop = messagesDiv.scrollHeight;
        }

        function displayChatHistory(contactUserId) {
            messagesDiv.innerHTML = ''; 
            appendMessage({ type: 'system', message: `Channel open with ${currentRecipient.username}.` });

            const history = allMessages[contactUserId] || [];
            history.forEach(appendMessage);
        }

        async function apiCall(endpoint, method, body = null) {
            try {
                const headers = { 'Content-Type': 'application/json' };

                // Endpoints that do NOT require a session key
                const publicEndpoints = ['register', 'login'];

                // Only add session key if currentUser exists AND the endpoint is not public
                if (currentUser && currentUser.session_token && !publicEndpoints.includes(endpoint)) {
                    headers['x-session-key'] = currentUser.session_token;
                }

                // If the endpoint *requires* a session key and it's missing, then throw an error.
                // This condition is now stricter and won't block initial register/login.
                if (!publicEndpoints.includes(endpoint) && (!currentUser || !currentUser.session_token)) {
                     console.warn(`Missing session token for endpoint ${endpoint}`);
                     showMessageBox('You are not logged in. Please sign in again.', true);
                     throw new Error('Session token missing for authenticated endpoint.');
                }

                const response = await fetch(`/${endpoint}`, {
                    method,
                    headers,
                    body: body ? JSON.stringify(body) : null
                });

                // Read the response body as text first
                const responseText = await response.text();
                let responseData = {};

                try {
                    // Attempt to parse the text as JSON
                    responseData = JSON.parse(responseText);
                } catch (jsonParseError) {
                    // If JSON parsing fails, use the raw text as the message, but log the error
                    console.error(`Failed to parse JSON response for ${endpoint}. Raw text: "${responseText.substring(0, 200)}..." Error: ${jsonParseError}`);
                    // Fallback to a generic error message or use a truncated raw text
                    responseData = { message: `Server responded with non-JSON format or error. Raw: ${responseText.substring(0, 100)}...` };
                }

                if (!response.ok) {
                    // If response is not OK, use the message from responseData or a generic one
                    const errorMessage = responseData.message || `HTTP error! status: ${response.status} ${response.statusText}.`;
                    throw new Error(errorMessage);
                }
                return responseData;
            } catch (error) {
                console.error(`API Call Error (${endpoint}):`, error);
                showMessageBox(error.message, true);
                throw error;
            }
        }


        // --- Event Handlers ---
        registerButton.addEventListener('click', async () => {
            const username = authUsernameInput.value.trim();
            const password = authPasswordInput.value.trim();
            if (!username || !password) return showMessageBox(`Callsign and Passkey are required.`, true);
            
            try {
                const response = await apiCall('register', 'POST', { username, password });
                handleLoginResponse(response); 
                showMessageBox(`Registration successful! You are now connected.`);
                authUsernameInput.value = '';
                authPasswordInput.value = '';
            } catch (error) { /* Error is handled by apiCall */ }
        });

        loginButton.addEventListener('click', async () => {
            const username = authUsernameInput.value.trim();
            const password = authPasswordInput.value.trim();
            if (!username || !password) return showMessageBox(`Callsign and Passkey are required.`, true);
            
            try {
                const response = await apiCall('login', 'POST', { username, password });
                handleLoginResponse(response); 
                showMessageBox('Connection re-established!');
                authUsernameInput.value = '';
                authPasswordInput.value = '';
            } catch (error) { /* Error is handled by apiCall */ }
        });

        logoutButton.addEventListener('click', clearCurrentUser);

        addContactButton.addEventListener('click', async () => {
            const contactUsername = addContactUsernameInput.value.trim();
            if (!contactUsername) return;
            if (currentUser && contactUsername === currentUser.username) {
                return showMessageBox("Cannot establish a connection with yourself!", true);
            }

            try {
                await apiCall('contacts', 'POST', { contact_username: contactUsername });
                showMessageBox(`Connection with ${contactUsername} established!`);
                addContactUsernameInput.value = '';
                fetchContacts();
            } catch (error) { /* Handled by apiCall */ }
        });

        sendButton.addEventListener('click', sendMessage);
        messageInput.addEventListener('keypress', (e) => { 
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                sendMessage(); 
            }
        });
        
        // Function to insert emoji at cursor position
        function insertEmoji(emoji) {
            const input = messageInput;
            const start = input.selectionStart;
            const end = input.selectionEnd;
            input.value = input.value.substring(0, start) + emoji + input.value.substring(end);
            input.selectionStart = input.selectionEnd = start + emoji.length;
            input.focus();
            // Trigger input event to send typing indicator if needed
            input.dispatchEvent(new Event('input', { bubbles: true }));
        }     
        function handleTypingStatus(data) {
            // Get references to the specific elements
            const typingStatusTextElement = typingIndicatorContainer.querySelector('#typingStatusText');
            const typingBarElement = typingIndicatorContainer.querySelector('.typing-bar');

            // Only show typing indicator for the currently selected recipient
            if (currentRecipient && data.from_user_id === currentRecipient.id) {
                if (data.is_typing) {
                    typingStatusTextElement.textContent = `${currentRecipient.username} is transmitting`;
                    // Make text and bar visible
                    typingStatusTextElement.style.display = 'inline'; /* or 'block' depending on desired layout */
                    typingBarElement.style.display = 'block';
                } else {
                    typingStatusTextElement.textContent = ''; // Clear text
                    // Hide text and bar
                    typingStatusTextElement.style.display = 'none';
                    typingBarElement.style.display = 'none';
                }
            } else {
                // If it's a typing indicator for a different contact or no contact selected, hide them
                typingStatusTextElement.textContent = '';
                typingStatusTextElement.style.display = 'none';
                typingBarElement.style.display = 'none';
            }
        }


        function handleChatMessage(data) {
            const isSelf = data.from_user_id === currentUser.user_id;
            const contactIdForHistory = isSelf ? data.to_user_id : data.from_user_id;

            if (!allMessages[contactIdForHistory]) {
                allMessages[contactIdForHistory] = [];
            }
            allMessages[contactIdForHistory].push(data);
            
            if (currentRecipient && contactIdForHistory === currentRecipient.id) {
                appendMessage(data);
            } else if (!isSelf) {
                showMessageBox(`Incoming transmission from ${data.from_username}!`);
            }
        }
        
        function handleStatusMessage(data) {
            const contactElem = document.querySelector(`.contact-button[data-user-id="${data.user_id}"] .status-indicator`);
            if (contactElem) {
                if(data.status === 'online') {
                    contactElem.classList.add('online');
                } else {
                    contactElem.classList.remove('online');
                }
            }
        }

        function sendMessage() {
            const message = messageInput.value.trim();
            if (!message || !currentRecipient) return;
            if (!ws || ws.readyState !== WebSocket.OPEN) {
                return showMessageBox(`Not connected to transmission server.`, true);
            }
            const chatMessage = {
                type: 'chatMessage', 
                to_user_id: currentRecipient.id,
                message: message
            };

            ws.send(JSON.stringify(chatMessage));
            messageInput.value = '';
            // Immediately send typing off when message is sent
            clearTimeout(typingTimeout);
            ws.send(JSON.stringify({ type: "typingIndicator", to_user_id: currentRecipient.id, is_typing: false }));
        }

        async function fetchContacts() {
            try {
                const contacts = await apiCall('contacts', 'GET');
                const noContactsMsg = document.getElementById('noContactsMessage');
                contactsListDiv.innerHTML = ''; // Clear existing contacts
                if (contacts.length === 0) {
                    contactsListDiv.appendChild(noContactsMsg);
                    noContactsMsg.classList.remove('hidden');
                } else {
                    if(noContactsMsg) noContactsMsg.classList.add('hidden');
                    contacts.forEach(contact => {
                        if (currentUser && contact.id === currentUser.user_id) return; // Don't list self as contact
                        const contactButton = document.createElement('button');
                        contactButton.className = 'contact-button';
                        contactButton.dataset.userId = contact.id;
                        contactButton.dataset.username = contact.username;
                        const nameSpan = document.createElement('span');
                        nameSpan.textContent = contact.username;
                        const statusIndicator = document.createElement('div');
                        statusIndicator.className = 'status-indicator';
                        contactButton.appendChild(nameSpan);
                        contactButton.appendChild(statusIndicator);
                        contactButton.addEventListener('click', () => selectContact(contact));
                        contactsListDiv.appendChild(contactButton);
                    });
                }
            } catch (error) { /* Error handled by apiCall */ }
        }

        function selectContact(contact) {
            // Hide typing indicator elements when switching contacts
            typingStatusText.textContent = '';
            typingStatusText.style.display = 'none';
            typingBar.style.display = 'none';

            document.querySelectorAll('.contact-button').forEach(btn => btn.classList.remove('active'));
            const btnElement = contactsListDiv.querySelector(`[data-user-id=\"${contact.id}\"]`);
            if (btnElement) btnElement.classList.add('active');
            
            currentRecipient = contact;
            chattingWithSpan.textContent = `Transmitting to: ${contact.username}`;
            enableChatInput(true);
            messageInput.focus();
            displayChatHistory(contact.id);
        }

        messageInput.addEventListener('input', () => {
            if (!currentRecipient || !ws || ws.readyState !== WebSocket.OPEN) return;
            
            // Send typing ON
            ws.send(JSON.stringify({ type: "typingIndicator", to_user_id: currentRecipient.id, is_typing: true }));

            // Reset timeout to send typing OFF
            clearTimeout(typingTimeout);
            typingTimeout = setTimeout(() => {
                ws.send(JSON.stringify({ type: "typingIndicator", to_user_id: currentRecipient.id, is_typing: false }));
            }, 2000); // Send typing OFF after 2 seconds of inactivity
        });

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


