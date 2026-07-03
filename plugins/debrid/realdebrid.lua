-- ~/.config/jadm/plugins/debrid/realdebrid.lua
-- Fill in your Real-Debrid API Key below:
local API_KEY = "YOUR_REAL_DEBRID_API_KEY_HERE"

jadm.register_debrid({
    name = "Real-Debrid",
    can_handle = function(url)
        return url:match("^magnet:") or url:match("%.torrent$")
    end,
    resolve = function(url)
        if API_KEY == "YOUR_REAL_DEBRID_API_KEY_HERE" or API_KEY == "" then
            print("[Real-Debrid] Error: API key is not configured in realdebrid.lua")
            return nil
        end

        local headers = {
            ["Authorization"] = "Bearer " .. API_KEY
        }

        -- 1. Add magnet link to Real-Debrid queue
        local body = "magnet=" .. url
        if url:match("%.torrent$") then
            body = "torrent=" .. url
        end

        local status, text = jadm.http.post("https://api.real-debrid.com/rest/1.0/torrents/addMagnet", body, headers)
        if status ~= 201 then
            print("[Real-Debrid] Failed to add magnet: " .. tostring(status) .. " - " .. tostring(text))
            return nil
        end

        -- Parse torrent ID from response: {"id":"XXXXX"}
        local torrent_id = text:match('"id"%s*:%s*"([^"]+)"')
        if not torrent_id then
            print("[Real-Debrid] Failed to parse torrent ID from: " .. tostring(text))
            return nil
        end

        -- 2. Select all files in the torrent to start the download
        local select_status, _ = jadm.http.post(
            "https://api.real-debrid.com/rest/1.0/torrents/selectFiles/" .. torrent_id,
            "files=all",
            headers
        )
        if select_status ~= 204 then
            print("[Real-Debrid] Failed to select files: " .. tostring(select_status))
            return nil
        end

        -- 3. Query the torrent info to get the download links
        -- Wait a moment for RD to fetch the metadata and generate download links
        local link = nil
        for attempt = 1, 5 do
            local info_status, info_text = jadm.http.get(
                "https://api.real-debrid.com/rest/1.0/torrents/info/" .. torrent_id,
                headers
            )
            if info_status == 200 then
                link = info_text:match('"links"%s*:%s*%[%s*"([^"]+)"')
                if link then
                    link = link:gsub("\\/", "/")
                    break
                end
            end
        end

        if not link then
            print("[Real-Debrid] Failed to obtain download links from torrent info")
            return nil
        end

        -- 4. Unrestrict the download link to get high-speed HTTP access
        local unrest_status, unrest_text = jadm.http.post(
            "https://api.real-debrid.com/rest/1.0/unrestrict/link",
            "link=" .. link,
            headers
        )
        if unrest_status == 200 then
            local download_url = unrest_text:match('"download"%s*:%s*"([^"]+)"')
            if download_url then
                download_url = download_url:gsub("\\/", "/")
                return download_url
            end
        end

        return nil
    end
})
