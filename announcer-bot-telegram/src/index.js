const { Telegraf } = require('telegraf');
//const { LCDClient } = require('@terra-money/terra.js');
const axios = require('axios');
const fs = require('fs');
const path = require('path');
const LocalSession = require('telegraf-session-local');


const botToken = process.env.TELEGRAM_TOKEN;
const LCD_URL = 'https://terra-classic-lcd.publicnode.com';

const dataPath = path.join(__dirname, 'data');
const chatsFilePath = path.join(dataPath, 'chats.json');
const storagePath = path.join(dataPath, 'storage.json');

if (!fs.existsSync(dataPath)) {
    fs.mkdirSync(dataPath);
}

let chats = [];
if (fs.existsSync(chatsFilePath)) {
    chats = JSON.parse(fs.readFileSync(chatsFilePath, 'utf8'));
}

let storage = {};
if (fs.existsSync(storagePath)) {
    storage = JSON.parse(fs.readFileSync(storagePath, 'utf8'));
}

function saveChats() {
    fs.writeFileSync(chatsFilePath, JSON.stringify(chats, null, 2));
}

function saveStorage() {
    fs.writeFileSync(storagePath, JSON.stringify(storage, null, 2));
}

const bot = new Telegraf(botToken);
const localSession = new LocalSession({ database: path.join(dataPath, 'session_db.json') });
bot.use(localSession.middleware());

bot.start((ctx) => {
    if (!chats.find(c => c.id === ctx.chat.id)) {
        chats.push({ id: ctx.chat.id, notify: false, thread_id: null });
        saveChats();
    }
    ctx.reply('Bot has started! Use /notify to begin seeing announcements.');
});

async function checkforAdmin(ctx) {
    if (['group', 'supergroup'].includes(ctx.chat.type)) {
        try {
            // Fetch the list of chat administrators
            const admins = await ctx.getChatAdministrators();
            
            // Check if the user sending the command is an admin
            const isAdmin = admins.some(admin => admin.user.id === ctx.from.id);

            return isAdmin;
            
        } catch (error) {
            console.error('Error fetching admins:', error);
            return false;
        }
    } else {
        return true;
    }

}

bot.command('help', (ctx) => {
    ctx.reply('Use /notify to get announcements. Use /stopnotify to end the subscription. You can limit notifications to a thread by using /notify <thread id>.');
});

bot.command('stopnotify', async (ctx) => {
    const admin = await checkforAdmin(ctx);

    if (!admin) {
        ctx.reply('You are not allowed to use this command.');
        return;
    }

    const use_id = (ctx.chat.id < 0 ? ctx.chat.id : ctx.from.id)
    const chat = chats.find(c => c.id === use_id);
    if (chat && chat.notify) {
        chat.notify = false;
        saveChats();
        ctx.reply('Will no longer notify this chat about new announcements.');
    } else {
        ctx.reply('This chat has no announcements enabled.');
    }
});

bot.command('notify', async (ctx) => {
    const admin = await checkforAdmin(ctx);
    if (!admin) {
        ctx.reply('You are not allowed to use this command.');
        return;
    }

    // get first parameter (thread id)
    const threadId = ctx.message.text.split(/[ ]+/)[1];

    const use_id = (ctx.chat.id < 0 ? ctx.chat.id : ctx.from.id)
    const chat = chats.find(c => c.id === use_id);
    if (chat && chat.notify) {
        ctx.reply('This chat is already receiving announcements.');
        return;
    }
    
    if (chat) {
        chat.notify = true;
        if(threadId) {
            chat.thread_id = threadId;
        } else {
            chat.thread_id = null;
        }
    } else {
        ctx.reply('This chat is not registered. Please use /start to register this chat.');
        return;
    }

    ctx.reply('This chat will now be notified about new announcements.');

    saveChats();
});

function getAnnouncements() {
    const contract = process.env.ANNOUNCEMENT_CONTRACT;
    if(!contract) {
        console.error('No announcement contract set.');
        return;
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

function escapeMarkdownV2(text) {
    if(!text) {
        return '';
    }
    const escapeChars = '_-[]()~>#+=|{}.!';
    return text.split('').map(char => escapeChars.includes(char) ? `\\${char}` : char).join('');
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

function handleAnnouncements() {
    getAnnouncements().then(announcements => {
        if(announcements && announcements.length > 0) {
            console.log('Got announcements:', announcements);
            processAnnouncements(announcements);
        }
    }).catch(error => {
        console.error('Error fetching announcements:', error);
    });
}

function broadcastToChat(chat, message) {
    console.log(`Broadcasting message to chat ${chat.id}`);

    bot.telegram.sendMessage(chat.id, message, { parse_mode: 'MarkdownV2', message_thread_id: (chat.thread_id ? chat.thread_id : null) }).catch(error => {
        console.error(`Failed to send message to chat ${chat.id}. Error:`, error);
        if(error.response && (error.response.statusCode === 403 || (error.response.statusCode === 400 && error.response.description === 'Bad Request: chat not found' ))) {
            console.log('Removing chat', chat.id);
            chats = chats.filter(c => c.id !== chat.id);
            saveChats();
        }
    });
}

setInterval(handleAnnouncements, 30000);
handleAnnouncements();

async function main() {
    while(true) {
        try {
            await bot.launch();
            console.log('Bot started successfully.');
        } catch (error) {
            console.error('Failed to launch bot. Retrying in 5 seconds.', error);
            await new Promise(resolve => setTimeout(resolve, 5000));
        }
    }
}

main();