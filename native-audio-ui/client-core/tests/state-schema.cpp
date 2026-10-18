// Pure app-local schema consumer through the unchanged public C ABI. No socket or playback.
#include "pokelinux_sounds_client.h"
#include <algorithm>
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>
#include <time.h>

static uint64_t work = 0;
static uint64_t now() {
    timespec t{};
    assert(clock_gettime(CLOCK_BOOTTIME, &t) == 0 && t.tv_sec >= 0 && t.tv_nsec >= 0 && t.tv_nsec < 1000000000);
    return uint64_t(t.tv_sec) * 1000000000ULL + uint64_t(t.tv_nsec);
}
static void bounded() { assert(now() < work); }
static void pass(const char *name, unsigned cases = 1) {
    bounded();
    std::printf("SOUNDS_STATE_SCHEMA_PASS %s cases=%u\n", name, cases);
    std::fflush(stdout);
}
static bool same(const PrSoundsClientIdV1 &value, const char *text) {
    return value.length <= 64 && value.length == std::strlen(text) && std::memcmp(value.bytes, text, value.length) == 0;
}
struct Client final {
    PrSoundsClient *p = nullptr;
    Client() { bounded(); assert(pr_sounds_client_new_v1(&p) == PR_SOUNDS_CLIENT_OK && p); }
    ~Client() { pr_sounds_client_free_v1(p); }
    Client(const Client &) = delete;
    Client &operator=(const Client &) = delete;
    PrSoundsClientViewV1 view() const {
        PrSoundsClientViewV1 out{};
        assert(pr_sounds_client_view_v1(p, &out) == PR_SOUNDS_CLIENT_OK && out.reserved == 0 && out.route_count <= 32);
        return out;
    }
    PrSoundsClientPacketV1 packet() {
        PrSoundsClientPacketV1 out{};
        assert(pr_sounds_client_prepare_v1(p, &out) == PR_SOUNDS_CLIENT_OK);
        assert(out.connection_epoch == view().connection_epoch && out.token > 0 && out.reserved == 0 && out.length > 0 && out.length <= 4096);
        return out;
    }
    void sent(const PrSoundsClientPacketV1 &packet) {
        assert(pr_sounds_client_sent_v1(p, packet.connection_epoch, packet.token, PR_SOUNDS_CLIENT_SEND_COMPLETE, packet.length) == PR_SOUNDS_CLIENT_OK);
    }
    void begin() {
        bounded();
        assert(pr_sounds_client_start_v1(p) == PR_SOUNDS_CLIENT_OK);
        const auto connecting = view();
        assert(connecting.stage == PR_SOUNDS_CLIENT_CONNECTING && !connecting.online && connecting.connection_epoch > 0);
        assert(connecting.request_until_boottime_ns > now());
        assert(pr_sounds_client_connected_v1(p, connecting.connection_epoch) == PR_SOUNDS_CLIENT_OK);
        const auto request = packet();
        assert(std::string(reinterpret_cast<const char *>(request.bytes), request.length) == "{\"offset\":0,\"op\":\"media_status\"}");
        assert(request.original_until_boottime_ns == connecting.request_until_boottime_ns);
        sent(request);
        assert(view().stage == PR_SOUNDS_CLIENT_RECEIVING);
    }
    int reply(const std::string &bytes) {
        bounded();
        assert(!bytes.empty() && bytes.size() <= 4096);
        return pr_sounds_client_reply_v1(p, view().connection_epoch, reinterpret_cast<const uint8_t *>(bytes.data()), uint32_t(bytes.size()));
    }
};
using Fields = std::vector<std::string>;
static std::string object(const Fields &fields) {
    std::string out = "{";
    for (size_t i = 0; i < fields.size(); ++i) { if (i) out += ','; out += fields[i]; }
    return out + '}';
}
static Fields stateFields(const std::string &preview = "\"idle\"", const std::string &settled = "true", const std::string &requested = "\"media-a7\"", const std::string &applied = "null") {
    return {"\"connected\":true", "\"available\":true", "\"pending\":false", "\"requested\":" + requested,
            "\"applied\":" + applied, "\"policy_settled\":" + settled, "\"preview\":" + preview};
}
static Fields statusFields(const std::string &state) {
    return {"\"result\":\"media_status\"", "\"state\":" + state,
            "\"routes\":[{\"id\":\"media-a7\",\"label\":\"Speaker\"},{\"id\":\"media-z2\",\"label\":\"Earpiece\"}]",
            "\"next_offset\":null", "\"can_choose\":true", "\"can_reconnect\":false"};
}
static std::string status(const Fields &state = stateFields()) { return object(statusFields(object(state))); }
static void accepted(const std::string &wire) {
    Client c; c.begin();
    assert(c.reply(wire) == PR_SOUNDS_CLIENT_OK);
    const auto v = c.view();
    assert(v.online && v.available && v.stage == PR_SOUNDS_CLIENT_IDLE && v.request_kind == PR_SOUNDS_CLIENT_NO_REQUEST);
    assert(v.route_count == 2 && same(v.routes[0].id, "media-a7") && same(v.routes[1].id, "media-z2"));
}
static void refused(const std::string &wire) {
    Client c; c.begin();
    assert(c.reply(wire) == PR_SOUNDS_CLIENT_REFUSED);
    const auto v = c.view();
    assert(v.stage == PR_SOUNDS_CLIENT_CLOSED && !v.online && !v.available && !v.can_choose && v.can_reconnect);
    assert(v.route_count == 0 && !v.requested_present && !v.applied_present && v.status == PR_SOUNDS_CLIENT_REPLY_REFUSED);
}
static void baseline() {
    Client c; c.begin();
    assert(c.reply("{\"result\":\"denied\"}") == PR_SOUNDS_CLIENT_OK);
    const auto v = c.view();
    assert(v.stage == PR_SOUNDS_CLIENT_IDLE && v.online && v.request_kind == PR_SOUNDS_CLIENT_NO_REQUEST && v.status == PR_SOUNDS_CLIENT_DENIED);
    assert(v.route_count == 0 && !v.applied_present && !v.can_choose);
    pass("known_public_API_denied_control");
}
static bool requiredIdle() {
    Client c; c.begin();
    const int actual = c.reply(status());
    if (actual != PR_SOUNDS_CLIENT_OK) {
        std::printf("SOUNDS_STATE_SCHEMA_ASSERT required_7_field_idle_acceptance expected=%d actual=%d available=true requested=media-a7 policy_settled=true preview=idle\n", PR_SOUNDS_CLIENT_OK, actual);
        std::fflush(stdout);
        return false;
    }
    const auto v = c.view();
    assert(v.stage == PR_SOUNDS_CLIENT_IDLE && v.online && v.available && v.route_count == 2);
    assert(v.requested_present && same(v.requested, "media-a7") && !v.applied_present);
    pass("required_7_field_idle_acceptance");
    return true;
}
static void sixEnums() {
    // These are explicit distinct wire values. Public playback-model projection is a later ABI task.
    for (const char *value : {"idle", "starting", "playing", "retiring", "completed", "failed"}) {
        const std::string preview = std::string("\"") + value + '"';
        accepted(status(stateFields(preview, "true", "\"media-a7\"", std::strcmp(value, "playing") == 0 ? "\"media-a7\"" : "null")));
    }
    accepted(status(stateFields("\"idle\"", "false")));
    pass("six_explicit_enums_and_false_policy_boolean", 7);
}
static void strictState() {
    unsigned cases = 0;
    const auto original = stateFields();
    for (size_t index = 0; index < original.size(); ++index) {
        auto fields = original; fields.erase(fields.begin() + index); refused(status(fields)); ++cases;
    }
    auto legacy = original; legacy.resize(5); refused(status(legacy)); ++cases; // No compatibility default.
    auto extra = original; extra.push_back("\"extra\":false"); refused(status(extra)); ++cases;
    for (const char *value : {"null", "0", "1", "\"true\"", "{}", "[]"}) {
        auto fields = original; fields[5] = std::string("\"policy_settled\":") + value; refused(status(fields)); ++cases;
    }
    for (const char *value : {"null", "true", "0", "{}", "[]", "\"\"", "\"Playing\"", "\"admitted\"", "\"unknown\""}) {
        auto fields = original; fields[6] = std::string("\"preview\":") + value; refused(status(fields)); ++cases;
    }
    for (size_t index : {size_t(0), size_t(1), size_t(2)}) {
        auto fields = original; const auto name = fields[index].substr(0, fields[index].find(':') + 1); fields[index] = name + "0"; refused(status(fields)); ++cases;
    }
    for (size_t index : {size_t(3), size_t(4)}) {
        auto fields = original; const auto name = fields[index].substr(0, fields[index].find(':') + 1); fields[index] = name + "false"; refused(status(fields)); ++cases;
    }
    assert(cases == 29);
    pass("strict_required_state_legacy5_types_unknowns", cases);
}
static void strictTop() {
    unsigned cases = 0;
    auto original = statusFields(object(stateFields()));
    for (size_t index = 0; index < original.size(); ++index) {
        auto fields = original; fields.erase(fields.begin() + index); refused(object(fields)); ++cases;
    }
    for (const char *field : {"\"policy_settled\":true", "\"preview\":\"playing\"", "\"extra\":null"}) {
        auto fields = original; fields.push_back(field); refused(object(fields)); ++cases;
    }
    for (const char *value : {"null", "[]", "false"}) {
        auto fields = original; fields[1] = std::string("\"state\":") + value; refused(object(fields)); ++cases;
    }
    auto fields = original; fields[4] = "\"can_choose\":1"; refused(object(fields)); ++cases;
    fields = original; fields[5] = "\"can_reconnect\":\"false\""; refused(object(fields)); ++cases;
    assert(cases == 14);
    pass("unchanged_six_field_status_envelope", cases);
}
static void routesAndApplied() {
    for (int which = 0; which < 3; ++which) {
        Client c; c.begin();
        const char *requested = which == 0 ? "null" : "\"media-a7\"";
        const char *applied = which == 2 ? "\"media-a7\"" : "null";
        assert(c.reply(status(stateFields("\"idle\"", "true", requested, applied))) == PR_SOUNDS_CLIENT_OK);
        const auto v = c.view();
        assert(v.route_count == 2 && v.routes[0].label.length == 7 && std::memcmp(v.routes[0].label.bytes, "Speaker", 7) == 0);
        assert(v.routes[1].label.length == 8 && std::memcmp(v.routes[1].label.bytes, "Earpiece", 8) == 0);
        assert(bool(v.requested_present) == (which > 0) && bool(v.applied_present) == (which == 2));
        if (v.requested_present) assert(same(v.requested, "media-a7"));
        if (v.applied_present) assert(same(v.applied, "media-a7"));
    }
    pass("catalogue_and_nullable_requested_applied_preserved", 3);
}
static void selected() {
    for (int fault = 0; fault < 4; ++fault) {
        Client c; c.begin(); assert(c.reply(status()) == PR_SOUNDS_CLIENT_OK);
        PrSoundsClientIdV1 route{}; route.length = 8; std::memcpy(route.bytes, "media-a7", route.length);
        assert(pr_sounds_client_choose_v1(c.p, &route) == PR_SOUNDS_CLIENT_OK);
        const auto request = c.packet();
        const std::string packet(reinterpret_cast<const char *>(request.bytes), request.length);
        assert(packet.find("\"op\":\"media_route\"") != std::string::npos && packet.find("\"route\":\"media-a7\"") != std::string::npos);
        c.sent(request);
        auto state = stateFields();
        if (fault == 1) state.resize(5);
        if (fault == 2) state[6] = "\"preview\":\"admitted\"";
        Fields top{"\"result\":\"media_selected\"", "\"state\":" + object(state)};
        if (fault == 3) top.push_back("\"preview\":\"idle\"");
        const auto code = c.reply(object(top));
        assert(code == (fault == 0 ? PR_SOUNDS_CLIENT_OK : PR_SOUNDS_CLIENT_REFUSED));
        const auto v = c.view();
        if (fault == 0) assert(v.stage == PR_SOUNDS_CLIENT_IDLE && v.online && v.requested_present && same(v.requested, "media-a7") && !v.applied_present);
        else assert(v.stage == PR_SOUNDS_CLIENT_CLOSED && !v.online && !v.requested_present && !v.applied_present);
    }
    pass("selected_reply_same_seven_field_state", 4);
}
int main() {
    const auto start = now(); work = start + 5000000000ULL;
    baseline();
    if (!requiredIdle()) return 101; // Intended old-engine functional RED; all following cases are frozen before repair.
    sixEnums(); strictState(); strictTop(); routesAndApplied(); selected();
    bounded();
    std::puts("SOUNDS_STATE_SCHEMA_ALL_PASS groups=7 schema_only=true public_playback_model=false");
}
