#pragma once
#include "SettingsConnectivityClient.h"
#include <QCoreApplication>
#include <QEventLoop>
#include <cassert>
#include <time.h>
#include <unistd.h>

inline void closeSettingsConnectivity(SettingsConnectivityClient &client) {
    const auto now = [] {
        timespec t{};
        assert(::clock_gettime(CLOCK_BOOTTIME, &t) == 0);
        return uint64_t(t.tv_sec) * 1000000000ULL + uint64_t(t.tv_nsec);
    };
    const uint64_t until = now() + 200000000ULL;
    int closed = 0;
    const auto connection = QObject::connect(&client, &SettingsConnectivityClient::closed,
                                            &client, [&] { ++closed; });
    client.beginClose();
    client.beginClose(); // Idempotent local close; never a second Stop.
    assert(closed == 0 && !client.canControl());
    while (!closed) {
        assert(now() < until);
        QCoreApplication::processEvents(QEventLoop::AllEvents, 1);
        if (!closed) ::usleep(1000);
    }
    assert(closed == 1 && now() < until);
    QObject::disconnect(connection);
}
