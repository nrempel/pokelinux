.pragma library

// The pinned Omarchy shell consumes a small line-based subset of TOML.
// This parser handles that appearance contract, not arbitrary TOML documents.
function entries(raw, sections) {
    const result = {}
    let section = ""
    for (const line of String(raw || "").split("\n")) {
        if (!sections) {
            const base = line.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*["']?(#[0-9a-f]{6})/i)
            if (base) result[base[1]] = base[2]
            continue
        }
        const heading = line.match(/^\s*\[([A-Za-z0-9_-]+)\]\s*(?:#.*)?$/)
        if (heading) { section = heading[1]; continue }
        const pair = line.match(/^\s*([A-Za-z0-9_-]+)\s*=\s*(?:"([^"\n]*)"|'([^'\n]*)'|([^#\n]+))\s*(?:#.*)?$/)
        if (!pair || (sections && !section)) continue
        const value = pair[2] !== undefined ? pair[2] : pair[3] !== undefined ? pair[3] : pair[4].trim()
        result[(sections ? section + "." : "") + pair[1]] = value
    }
    return result
}
function hex(value) {
    return /^#[0-9a-f]{6}$/i.test(value || "") ? value : null
}
function color(value) {
    let token = String(value || "").trim()
    let m = token.match(/^#([0-9a-f]{3})$/i)
    if (m) token = "#" + m[1].split("").map(x => x + x).join("")
    m = token.match(/^(?:#|rgba?\()([0-9a-f]{6})([0-9a-f]{2})?\)?$/i)
    if (m) return Qt.rgba(parseInt(m[1].slice(0,2),16)/255,
        parseInt(m[1].slice(2,4),16)/255,parseInt(m[1].slice(4,6),16)/255,
        m[2] ? parseInt(m[2],16)/255 : 1)
    m = token.match(/^0x([0-9a-f]{2})([0-9a-f]{6})$/i)
    if (m) return color("#" + m[2] + m[1])
    m = token.match(/^rgba?\((\d+),(\d+),(\d+)(?:,([\d.]+))?\)$/i)
    if (m && (m[4] === undefined || isFinite(Number(m[4]))))
        return Qt.rgba(Math.min(255,Number(m[1]))/255,Math.min(255,Number(m[2]))/255,
            Math.min(255,Number(m[3]))/255,m[4] === undefined ? 1 : Math.max(0,Math.min(1,Number(m[4]))))
    return token === "transparent" ? Qt.rgba(0,0,0,0) : null
}
function flat(value, roles, shell, fallback, seen) {
    const tokens = String(value || "").trim().split(/\s+/).filter(x => !/^-?\d+(?:\.\d+)?deg$/.test(x))
    const token = tokens[0] || ""
    const key = token.toLowerCase()
    if (seen.length >= 16 || seen.indexOf(key) >= 0) return fallback
    if (typeof shell[key] === "string") return flat(shell[key],roles,shell,fallback,seen.concat(key))
    if (typeof roles[key] === "string") return color(roles[key]) || fallback
    return color(token) || fallback
}
function composite(base, alpha, background) {
    const a = Math.max(0,Math.min(1,alpha)) * base.a
    return Qt.rgba((Math.round(255*(base.r*a+background.r*(1-a))))/255,
        Math.round(255*(base.g*a+background.g*(1-a)))/255,
        Math.round(255*(base.b*a+background.b*(1-a)))/255,1)
}
function palette(snapshot) {
    const base = entries(snapshot.colors,false)
    const bg = hex(base.background) || hex(base.color0)
    const fg = hex(base.foreground) || hex(base.color7)
    const accent = hex(base.accent) || hex(base.color4) || fg
    if (!bg || !fg) return null
    let urgent = accent
    for (const key in base) if ((key === "red" || key === "color1") && hex(base[key])) urgent = base[key]
    const roles = {background:bg,foreground:fg,text:fg,accent:accent,
        muted:hex(base.muted)||hex(base.color8)||fg,
        urgent:urgent}
    const shell = entries(snapshot.shell,true)
    const user = entries(snapshot.userShell,true)
    for (const key in user) shell[key] = user[key]
    function surface(key, fallback) {
        const value = flat(shell[key],roles,shell,color(fallback),[])
        const a = shell[key+"-alpha"] === undefined ? 1 : Number(shell[key+"-alpha"])
        return composite(value,isFinite(a)?a:1,color(bg))
    }
    const parsedSize = parseInt(shell["font.base-size"],10)
    const size = isFinite(parsedSize) ? Math.max(1,parsedSize) : 12
    // Match original Commons.Style's per-token sizes, not app-specific scales.
    function fontToken(key,mult) {
        const value = parseInt(shell["font."+key],10)
        return isFinite(value) && value > 0 ? Math.round(value) : Math.max(1,Math.round(size*mult))
    }
    const fonts = {caption:fontToken("caption",0.833),bodySmall:fontToken("body-small",0.917),
        body:fontToken("body",1),subtitle:fontToken("subtitle",1.083),title:fontToken("title",1.167),
        heading:fontToken("heading",1.333),display:fontToken("display",2),displayLarge:fontToken("display-large",2.333)}
    const spacingRaw = parseFloat(shell["spacing.scale"])
    const spacingScale = isFinite(spacingRaw) && spacingRaw >= 0 ? spacingRaw : 1
    const withFont = !/^(false|0|no|off)$/i.test(String(shell["spacing.scale-with-font"]).trim())
    const scale = spacingScale * (withFont ? size/12 : 1)
    function spacingToken(key,fallback) {
        const value = parseFloat(shell["spacing."+key])
        return isFinite(value) && value >= 0 ? Math.round(value) : (scale===0 ? 0 : Math.max(1,Math.round(fallback*scale)))
    }
    const spacing = {controlGap:spacingToken("control-gap",8),panelPadding:spacingToken("panel-padding",18),
        panelGap:spacingToken("panel-gap",14),rowGap:spacingToken("row-gap",8),
        controlPaddingX:spacingToken("control-padding-x",10),controlPaddingY:spacingToken("control-padding-y",6)}
    return {background:bg,foreground:surface("popups.text",fg),
        surface:surface("popups.background",bg),border:surface("popups.border",accent),
        accent:accent,muted:roles.muted,urgent:roles.urgent,
        positive:hex(base.green)||hex(base.color2)||accent,ink:bg,
        fontScale:size/12,fontTokens:fonts,spacingTokens:spacing,
        fontFamily:String(snapshot.resolvedFontFamily || "monospace")}
}
