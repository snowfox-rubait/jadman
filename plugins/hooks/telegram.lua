-- ~/.config/jadm/plugins/hooks/telegram.lua
-- A lightweight Telegram remote Snatcher Bot.
-- Long-polls the Telegram API to listen for magnet links or URLs and queues them.

local BOT_TOKEN = "YOUR_TELEGRAM_BOT_TOKEN_HERE"
local ALLOWED_CHAT_ID = 0 -- Set to your chat ID to restrict access

local function handle_message(text, chat_id)
    if ALLOWED_CHAT_ID ~= 0 and chat_id ~= ALLOWED_CHAT_ID then
        return
    end

    local url = text:match("(magnet:[%S]+)") or text:match("(https?://[%S]+)")
    if url then
        print("[JADMan Telegram Bot] Snatching URL: " .. url)
        jadm.add_download(url)
        jadm.notify("Telegram Snatch", "Queued download from Telegram: " .. url)
    end
end

jadm.register_loop(function()
    if BOT_TOKEN == "YOUR_TELEGRAM_BOT_TOKEN_HERE" or BOT_TOKEN == "" then
        print("[JADMan Telegram Bot] Telegram token not configured in telegram.lua. Skipping...")
        return
    end

    local last_update_id = 0
    while true do
        local url = "https://api.telegram.org/bot" .. BOT_TOKEN .. "/getUpdates?offset=" .. (last_update_id + 1) .. "&timeout=30"
        local status, text = jadm.http.get(url)
        
        if status == 200 then
            for update in text:gmatch('(%b{})') do
                local update_id_str = update:match('"update_id"%s*:%s*(%d+)')
                if update_id_str then
                    local update_id = tonumber(update_id_str)
                    if update_id and update_id > last_update_id then
                        last_update_id = update_id
                    end
                end
                
                local chat_id_str = update:match('"chat"%s*:%s*%b{}')
                if chat_id_str then
                    local chat_id = tonumber(chat_id_str:match('"id"%s*:%s*(%-?%d+)'))
                    local text_msg = update:match('"text"%s*:%s*"([^"]+)"')
                    if chat_id and text_msg then
                        text_msg = text_msg:gsub("\\/", "/")
                        handle_message(text_msg, chat_id)
                    end
                end
            end
        end
        jadm.sleep(2000)
    end
end)
