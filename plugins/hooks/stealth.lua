-- ~/.config/jadm/plugins/hooks/stealth.lua
-- Monitors latency and dynamically throttles speed if latency > 120ms.

local NORMAL_LIMIT = 0 -- 0 means unlimited
local THROTTLED_LIMIT = 500 * 1024 -- 500 KB/s limit
local CHECK_INTERVAL_MS = 10000 -- Every 10 seconds

jadm.register_loop(function()
    while true do
        local start_time = os.clock()
        local status, _ = jadm.http.get("https://1.1.1.1")
        local end_time = os.clock()
        
        if status == 200 then
            local rtt_ms = (end_time - start_time) * 1000
            if rtt_ms > 120 then
                print("[JADMan Stealth] High network latency detected: " .. string.format("%.1f", rtt_ms) .. "ms. Throttling downloads to 500KB/s...")
                jadm.set_limit(THROTTLED_LIMIT)
            else
                jadm.set_limit(NORMAL_LIMIT)
            end
        end
        jadm.sleep(CHECK_INTERVAL_MS)
    end
end)
