-- ~/.config/jadm/plugins/recovery/wayback.lua

local function url_encode(str)
    if str then
        str = str:gsub("\n", "\r\n")
        str = str:gsub("([^%w %-%_%.%~])", function(c)
            return string.format("%%%02X", string.byte(c))
        end)
        str = str:gsub(" ", "+")
    end
    return str
end

jadm.register_recovery({
    name = "Wayback Machine",
    can_handle = function(url, error_code)
        return (url:match("^http://") or url:match("^https://")) and error_code == 404
    end,
    resolve = function(url, error_code)
        local api_url = "https://archive.org/wayback/available?url=" .. url_encode(url)
        local status, text = jadm.http.get(api_url)
        if status == 200 then
            local archive_url = text:match('"url"%s*:%s*"([^"]+)"')
            if archive_url then
                archive_url = archive_url:gsub("\\/", "/")
                return archive_url
            end
        end
        return nil
    end
})
