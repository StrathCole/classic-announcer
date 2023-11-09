const { Client, GatewayIntentBits, Partials, ChannelType, PermissionFlagsBits } = require('discord.js');
const fs = require('fs');
const path = require('path');
const axios = require('axios');

const botToken = process.env.DISCORD_TOKEN;
const client = new Client({ intents: [ GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.DirectMessages,
    GatewayIntentBits.MessageContent
], partials: [Partials.Channel] });

const LCD_URL = 'https://terra-classic-lcd.publicnode.com';
const dataPath = path.join(__dirname, 'data');
const chatsFilePath = path.join(dataPath, 'chats.json');
const storagePath = path.join(dataPath, 'storage.json');

// Ensure data directory exists
if (!fs.existsSync(dataPath)) {
    fs.mkdirSync(dataPath);
}

// Load or initialize chats array
let chats = [];
if (fs.existsSync(chatsFilePath)) {
    chats = JSON.parse(fs.readFileSync(chatsFilePath, 'utf8'));
} else {
    saveChats();
}

// Load or initialize storage object
let storage = {};
if (fs.existsSync(storagePath)) {
    storage = JSON.parse(fs.readFileSync(storagePath, 'utf8'));
} else {
    saveStorage();
}

client.once('ready', () => {
    console.log('Discord bot is ready!');
});

client.on('messageCreate', async (message) => {
    if (message.author.bot) return; // Ignore bot's own messages

    const content = message.content.trim();
    if (!content.startsWith('/')) return; // Ignore non-commands

    const chatId = message.channel.type === 'DM' ? message.author.id : message.channel.id;
    if (!chats.find(c => c.id === chatId)) {
        chats.push({ id: chatId, notify: false, thread_id: null });
        saveChats();
    }

    const args = content.slice(1).split(/ +/);
    const command = args.shift().toLowerCase();

    switch (command) {
        case 'start':
            // Equivalent to Telegram's /start
            message.channel.send('Bot has started! Use /notify to begin seeing announcements.');
            break;
        case 'notify':
            // Equivalent to Telegram's /notify
            const isAdminNotify = await checkForAdmin(message);
            if (!isAdminNotify) {
                message.channel.send('You are not allowed to use this command.');
                return;
            }

            const threadId = args[0];

            const chatNotify = chats.find(c => c.id === chatId);
            if (chatNotify && chatNotify.notify) {
                message.channel.send('This chat is already receiving announcements.');
                return;
            }

            if (chatNotify) {
                chatNotify.notify = true;
                chatNotify.thread_id = threadId || null;
                message.channel.send('This chat will now be notified about new announcements.');
            } else {
                message.channel.send('This chat is not registered. Please use /start to register this chat.');
            }
            
            saveChats();
            break;
        case 'stopnotify':
            // Equivalent to Telegram's /stopnotify
            const isAdmin = await checkForAdmin(message);
            if (!isAdmin) {
                message.channel.send('You are not allowed to use this command.');
                return;
            }

            const chat = chats.find(c => c.id === chatId);
            if (chat && chat.notify) {
                chat.notify = false;
                saveChats();
                message.channel.send('Will no longer notify this chat about new announcements.');
            } else {
                message.channel.send('This chat has no announcements enabled.');
            }
            break;
        case 'help':
            // Equivalent to Telegram's /help
            message.channel.send('Use `/notify` to get announcements. Use `/stopnotify` to end the subscription.');
            break;
    }
});

function escapeMarkdownV2(text) {
    if(!text) {
        return '';
    }
    const escapeChars = '_-[]()~>#+=|{}.!';
    return text.split('').map(char => escapeChars.includes(char) ? `\\${char}` : char).join('');
}

async function checkForAdmin(message) {
    // DMs are not part of a guild, so assume the user is an "admin" of their DMs
    console.log('Check admin', message);
    if (message.channel.type === ChannelType.DM) return true;

    // In guilds, check if the member has the 'MANAGE_GUILD' permission
    return message.member.permissions.has(PermissionFlagsBits.ManageGuild);
}

function saveChats() {
    fs.writeFileSync(chatsFilePath, JSON.stringify(chats, null, 2));
}

function saveStorage() {
    fs.writeFileSync(storagePath, JSON.stringify(storage, null, 2));
}

function getAnnouncements() {
    const contract = process.env.ANNOUNCEMENT_CONTRACT;
    if(!contract) {
        console.error('No announcement contract set.');
        throw new Error('No announcement contract set.');
    }

    const jsonData = {
        announcements: {
            since: (storage?.lastAnnouncement ? storage.lastAnnouncement.time : null)
        }
    };

    const base64EncodedData = Buffer.from(JSON.stringify(jsonData)).toString('base64');

    return axios.get(`${LCD_URL}/cosmwasm/wasm/v1/contract/${contract}/smart/${base64EncodedData}`)
        .then(response => {
            return response.data.data;
        })
        .catch(error => {
            console.error("Error fetching data:", error);
        });
}


function processAnnouncements(announcements) {
    const relevantChats = chats.filter(c => c.notify);
    if(relevantChats.length < 1) {
        console.log('no chats');
        return;
    }

    let message = '';
    for (let announcement of announcements) {
        if(announcement.id <= storage.lastAnnouncement?.id) {
            continue;
        }
        console.log('Sending announcement to', relevantChats.length, 'chats.');
        console.log('Announcement:', announcement);
        storage.lastAnnouncement = announcement;

        message = `**${escapeMarkdownV2(announcement.title)}**\n\n${escapeMarkdownV2(announcement.content)}`;
        message += `\n\nTopic: ${escapeMarkdownV2(announcement.topic?.name)}`;
        if(announcement.time) {
            message += `\nSent: ${escapeMarkdownV2(new Date(announcement.time / 1000000).toLocaleString())}`;
        }

        for (let chat of relevantChats) {
            broadcastToChat(chat, message);
        }
    }

    saveStorage();
}

async function handleAnnouncements() {
    getAnnouncements().then(announcements => {
        if(announcements && announcements.length > 0) {
            console.log('Got announcements:', announcements);
            processAnnouncements(announcements);
        }
    }).catch(error => {
        console.error('Error fetching announcements:', error);
    });
}

async function broadcastToChat(chat, message) {
    console.log(`Broadcasting message to chat ${chat.id}`);

    let channel;
    try {
        channel = await client.channels.fetch(chat.id);
    } catch (error) {
        console.error(`Failed to fetch channel with ID ${chat.id}:`, error);
        return;
    }

    // If chat.thread_id is set, we try to send the message to the thread.
    if (chat.thread_id) {
        let thread;
        try {
            thread = await channel.threads.fetch(chat.thread_id);
        } catch (error) {
            console.error(`Failed to fetch thread with ID ${chat.thread_id}:`, error);
            return;
        }

        // If the thread is archived, unarchive it before sending the message
        if (thread.archived) {
            console.error(`Thread with ID ${chat.thread_id} is archived.`);
            return;
        }

        try {
            await thread.send(message);
        } catch (error) {
            console.error(`Failed to send message to thread with ID ${chat.thread_id}:`, error);
            // Handle specific cases like if the bot was removed from the thread or blocked by the user.
        }
    } else {
        // If chat.thread_id is not set, send the message to the channel.
        try {
            await channel.send(message);
        } catch (error) {
            console.error(`Failed to send message to channel or DM with ID ${chat.id}:`, error);
            // Handle specific cases like if the bot was removed from the channel or blocked by the user.
        }
    }
}


// Set an interval for the handleAnnouncements function
setInterval(handleAnnouncements, 30000);
handleAnnouncements();

// Login to Discord with your bot's token
client.login(botToken);

