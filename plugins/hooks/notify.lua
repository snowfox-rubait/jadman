-- ~/.config/jadm/plugins/hooks/notify.lua
jadm.register_hook("on_download_complete", function(download)
    jadm.notify("JADMan Completed", download.filename or "Download finished successfully")
end)
