#include <QtTest>
#include <QTemporaryDir>
#include <QFile>
#include <QDir>
#include <QSaveFile>
#include "PhoneThemeFiles.h"

class ThemeFilesTest : public QObject {
    Q_OBJECT
    static void write(const QString &path, const QByteArray &body) {
        QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
        QSaveFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        QCOMPARE(file.write(body), body.size());
        QVERIFY(file.commit());
    }
    static QString current(const QString &home) {
        return home + "/.local/state/omarchy/current/theme";
    }
private slots:
    void readsInstalledPathsAndOptionalSettings() {
        QTemporaryDir home; QVERIFY(home.isValid());
        write(current(home.path()) + "/colors.toml", "background = '#101010'\n");
        PhoneThemeFiles files(home.path());
        QVERIFY(files.available());
        QCOMPARE(files.snapshot().value("colors").toString(), "background = '#101010'\n");
        QCOMPARE(files.snapshot().value("shell").toString(), "");
        QCOMPARE(files.snapshot().value("userShell").toString(), "");
        write(home.path() + "/.config/omarchy/shell.toml", "[font]\nfamily = 'Test Font'\n");
        QTRY_COMPARE(files.snapshot().value("userShell").toString(), "[font]\nfamily = 'Test Font'\n");
    }
    void followsSelectionAndFileReplacement() {
        QTemporaryDir home; QVERIFY(home.isValid());
        const auto selected = current(home.path());
        const auto first = home.path() + "/themes/first";
        const auto second = home.path() + "/themes/second";
        write(first + "/colors.toml", "background = '#111111'\n");
        write(first + "/shell.toml", "[popups]\nborder = 'accent'\n");
        write(second + "/colors.toml", "background = '#eeeeee'\n");
        write(second + "/shell.toml", "[popups]\nborder = 'foreground'\n");
        QVERIFY(QDir().mkpath(QFileInfo(selected).absolutePath()));
        QVERIFY(QFile::link(first, selected));
        PhoneThemeFiles files(home.path());
        QSignalSpy changes(&files, &PhoneThemeFiles::changed);
        QVERIFY(files.available());
        QVERIFY(QFile::remove(selected));
        QVERIFY(QFile::link(second, selected));
        QTRY_COMPARE(files.snapshot().value("colors").toString(), "background = '#eeeeee'\n");
        QCOMPARE(files.snapshot().value("shell").toString(), "[popups]\nborder = 'foreground'\n");
        QVERIFY(!changes.isEmpty());
        write(second + "/colors.toml", "background = '#dddddd'\n");
        QTRY_COMPARE(files.snapshot().value("colors").toString(), "background = '#dddddd'\n");
    }
    void preservesSnapshotDuringMissingOrInvalidInput() {
        QTemporaryDir home; QVERIFY(home.isValid());
        const auto path = current(home.path()) + "/colors.toml";
        write(path, "background = '#111111'\n");
        PhoneThemeFiles files(home.path());
        const auto good = files.snapshot();
        QVERIFY(QFile::remove(path));
        QTRY_VERIFY(!files.available());
        QCOMPARE(files.snapshot(), good);
        write(path, QByteArray(256 * 1024 + 1, 'x'));
        QTest::qWait(150);
        QVERIFY(!files.available());
        QCOMPARE(files.snapshot(), good);
        write(path, QByteArray::fromHex("fffe"));
        QTest::qWait(150);
        QVERIFY(!files.available());
        QCOMPARE(files.snapshot(), good);
        // An incomplete final UTF-8 sequence is invalid too, not a partial
        // streaming chunk that may be completed by a later file read.
        write(path, QByteArray::fromHex("6261636b67726f756e643de282"));
        QTest::qWait(150);
        QVERIFY(!files.available());
        QCOMPARE(files.snapshot(), good);
        write(path, "background = '#222222'\n");
        QTRY_VERIFY(files.available());
        QCOMPARE(files.snapshot().value("colors").toString(), "background = '#222222'\n");
    }
    void discoversMissingDirectoryAndOverrideDeletion() {
        QTemporaryDir home; QVERIFY(home.isValid());
        PhoneThemeFiles files(home.path());
        QVERIFY(!files.available());
        QVERIFY(files.snapshot().isEmpty());
        write(current(home.path()) + "/colors.toml", "background = '#121212'\n");
        QTRY_VERIFY(files.available());
        const auto overridePath = home.path() + "/.config/omarchy/shell.toml";
        write(overridePath, "[font]\nsize = 14\n");
        QTRY_VERIFY(!files.snapshot().value("userShell").toString().isEmpty());
        QVERIFY(QFile::remove(overridePath));
        QTRY_COMPARE(files.snapshot().value("userShell").toString(), "");
    }
};
QTEST_GUILESS_MAIN(ThemeFilesTest)
#include "test_theme_files.moc"
