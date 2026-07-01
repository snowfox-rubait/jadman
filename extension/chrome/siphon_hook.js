(function() {
    const DEBUG = true;

    // Retrieve and remove the communication token from documentElement
    let communicationToken = null;
    if (document.documentElement) {
        communicationToken = document.documentElement.getAttribute('data-jadman-token');
        if (communicationToken) {
            document.documentElement.removeAttribute('data-jadman-token');
        }
    }

    function log(...args) {
        if (DEBUG) console.log('[JADMan Siphon]', ...args);
    }

    function shouldBypass() {
        const mode = document.documentElement.getAttribute('data-jadman-mode');
        const paused = document.documentElement.getAttribute('data-jadman-paused');
        return paused === 'true' || mode === 'general';
    }

    function dispatch(url, type, data, mime) {
        if (shouldBypass()) return;
        if (!communicationToken) return;
        let priority = 'NORMAL';
        const lowerUrl = url.toLowerCase();
        
        if (type === 'MEDIA_DETECTED') {
            priority = 'MEDIA';
        } else if (lowerUrl.includes('.m3u8') || lowerUrl.includes('.mpd') || mime === 'application/x-mpegURL' || mime === 'application/dash+xml') {
            priority = 'MANIFEST';
        } else if (lowerUrl.includes('.ts') || lowerUrl.includes('.m4s') || lowerUrl.includes('.m4a') || lowerUrl.includes('.m4v')) {
            priority = 'SEGMENT';
        }

        const eventData = {
            url: url,
            type: type,
            priority: priority,
            data: data, 
            mime: mime
        };

        window.dispatchEvent(new CustomEvent(communicationToken, { detail: eventData }));
    }

    // STEALTH PROXY UTILITY FOR HOOKS
    const proxyOriginalMap = new WeakMap();
    const originalToString = Function.prototype.toString;

    Function.prototype.toString = new Proxy(originalToString, {
        apply(target, thisArg, argArray) {
            if (proxyOriginalMap.has(thisArg)) {
                const original = proxyOriginalMap.get(thisArg);
                return originalToString.call(original);
            }
            return originalToString.apply(thisArg, argArray);
        }
    });

    function createStealthProxy(original, hookImpl) {
        const handler = {
            apply(target, thisArg, argArray) {
                return hookImpl.apply(thisArg, argArray);
            },
            construct(target, argArray, newTarget) {
                return Reflect.construct(hookImpl, argArray, newTarget);
            },
            get(target, prop, receiver) {
                if (prop === 'toString') {
                    return function toString() {
                        return originalToString.call(original);
                    };
                }
                const val = Reflect.get(target, prop, receiver);
                return typeof val === 'function' ? val.bind(target) : val;
            }
        };
        const proxy = new Proxy(original, handler);
        proxyOriginalMap.set(proxy, original);
        return proxy;
    }

    // SHADOW DOM CAPTURE
    const pageShadowRoots = [];
    const originalAttachShadow = Element.prototype.attachShadow;
    Element.prototype.attachShadow = createStealthProxy(originalAttachShadow, function(options) {
        const shadow = originalAttachShadow.apply(this, arguments);
        pageShadowRoots.push(shadow);
        return shadow;
    });

    // FORCE PRESERVE DRAWING BUFFER FOR WEBGL (WebGL Canvas Capture support)
    const originalGetContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = createStealthProxy(originalGetContext, function(type, attributes) {
        if (type === 'webgl' || type === 'webgl2' || type === 'experimental-webgl') {
            attributes = attributes || {};
            attributes.preserveDrawingBuffer = true;
        }
        return originalGetContext.call(this, type, attributes);
    });

    // HIJACK FETCH
    const originalFetch = window.fetch;
    window.fetch = createStealthProxy(originalFetch, async function(...args) {
        if (shouldBypass()) {
            return originalFetch.apply(this, args);
        }
        const response = await originalFetch.apply(this, args);
        const url = response.url;
        const lowerUrl = url.toLowerCase();

        const isManifest = lowerUrl.includes('.m3u8') || lowerUrl.includes('.mpd');
        const isSegment = lowerUrl.includes('.ts') || lowerUrl.includes('.m4s');

        if (isManifest || isSegment) {
            const clone = response.clone();
            clone.arrayBuffer().then(buffer => {
                dispatch(url, 'FETCH', buffer, clone.headers.get('content-type'));
            }).catch(() => {});
        }

        return response;
    });

    // HIJACK XHR
    const originalOpen = XMLHttpRequest.prototype.open;
    const originalSend = XMLHttpRequest.prototype.send;

    XMLHttpRequest.prototype.open = createStealthProxy(originalOpen, function(method, url) {
        if (shouldBypass()) {
            return originalOpen.apply(this, arguments);
        }
        this._url = url;
        return originalOpen.apply(this, arguments);
    });

    XMLHttpRequest.prototype.send = createStealthProxy(originalSend, function() {
        if (shouldBypass()) {
            return originalSend.apply(this, arguments);
        }
        this.addEventListener('load', function() {
            try {
                if (shouldBypass()) return;
                const url = this.responseURL || this._url;
                const lowerUrl = url.toLowerCase();
                const contentType = this.getResponseHeader('content-type') || "";
                
                const isManifest = lowerUrl.includes('.m3u8') || lowerUrl.includes('.mpd') || contentType.includes('mpegURL') || contentType.includes('dash+xml');
                const isSegment = lowerUrl.includes('.ts') || lowerUrl.includes('.m4s');

                if (isManifest || isSegment) {
                    let data = (this.responseType === 'arraybuffer' || this.responseType === 'blob') ? this.response : this.responseText;
                    if (data) {
                        dispatch(url, 'XHR', data, contentType);
                    }
                }
            } catch (e) {}
        });
        return originalSend.apply(this, arguments);
    });

    // HIJACK MEDIASOURCE & SOURCEBUFFER
    try {
        if (window.MediaSource) {
            const originalAddSourceBuffer = MediaSource.prototype.addSourceBuffer;
            MediaSource.prototype.addSourceBuffer = createStealthProxy(originalAddSourceBuffer, function(mimeType) {
                if (shouldBypass()) {
                    return originalAddSourceBuffer.apply(this, arguments);
                }
                const sourceBuffer = originalAddSourceBuffer.call(this, mimeType);
                sourceBuffer._mimeType = mimeType;
                log('Hooked addSourceBuffer with mimeType:', mimeType);
                return sourceBuffer;
            });
        }

        if (window.SourceBuffer) {
            const originalAppendBuffer = SourceBuffer.prototype.appendBuffer;
            SourceBuffer.prototype.appendBuffer = createStealthProxy(originalAppendBuffer, function(data) {
                if (shouldBypass()) {
                    return originalAppendBuffer.apply(this, arguments);
                }
                try {
                    const mime = this._mimeType || 'video/mp4';
                    let buffer;
                    if (data instanceof ArrayBuffer) {
                        buffer = data;
                    } else if (data && data.buffer instanceof ArrayBuffer) {
                        buffer = data.buffer;
                    }

                    if (buffer && buffer.byteLength > 0) {
                        const isRecordMode = document.documentElement.getAttribute('data-jadman-record') === 'true';
                        dispatch(window.location.href, 'APPEND_BUFFER', isRecordMode ? buffer.slice(0) : null, mime);
                    }
                } catch (e) {
                    log('Error in appendBuffer hook:', e);
                }
                return originalAppendBuffer.apply(this, arguments);
            });
        }
    } catch (e) {
        log('Failed to install MediaSource hooks:', e);
    }

    // RECURSIVE SHADOW DOM SCANNER
    function findElements(root, tagName, list = []) {
        if (!root) return list;
        try {
            const elements = root.querySelectorAll(tagName);
            for (let i = 0; i < elements.length; i++) {
                list.push(elements[i]);
            }
        } catch(e) {}
        try {
            const all = root.querySelectorAll('*');
            for (let i = 0; i < all.length; i++) {
                const el = all[i];
                if (el.shadowRoot) {
                    findElements(el.shadowRoot, tagName, list);
                }
            }
        } catch(e) {}
        if (root === document) {
            for (let i = 0; i < pageShadowRoots.length; i++) {
                findElements(pageShadowRoots[i], tagName, list);
            }
        }
        return list;
    }

    let mediaIdCounter = 0;
    function scanAndTagMedia() {
        if (shouldBypass()) return;
        const videos = findElements(document, 'video');
        const audios = findElements(document, 'audio');
        const allMedia = [...videos, ...audios];
        
        for (let i = 0; i < allMedia.length; i++) {
            const el = allMedia[i];
            let mediaId = el.getAttribute('data-jadman-media-id');
            if (!mediaId) {
                mediaId = 'm_' + (++mediaIdCounter) + '_' + Math.random().toString(36).substring(2, 7);
                el.setAttribute('data-jadman-media-id', mediaId);
            }
            const src = el.src || el.currentSrc || '';
            dispatch(window.location.href, 'MEDIA_DETECTED', { mediaId: mediaId, src: src, tagName: el.tagName });
        }
    }
    setInterval(scanAndTagMedia, 2000);

    log('Deep Capture Hook (v2: Stream Aware + Shadow DOM) Active.');
})();
