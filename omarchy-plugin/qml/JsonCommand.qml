import QtQuick
import Quickshell.Io

// A separate job object owns each child. Old callbacks cannot finish a new job.
Item {
    id: root
    property int timeoutMs: 4000
    property var job: null
    readonly property bool active: job !== null
    signal completed(var context, bool ok, var value)
    function start(arguments, context) {
        if (active || !Array.isArray(arguments) || arguments.length === 0) return false;
        job=jobComponent.createObject(root,{argv:arguments.slice(),context:context});
        return job !== null;
    }
    Component {
        id: jobComponent
        Item {
            id: worker
            property var argv: []
            property var context: null
            property bool done: false
            property bool sawExit: false
            property bool sawEof: false
            property int exitCode: -1
            property int exitStatus: -1
            property string output: ""
            function finish(ok,value) {
                if (done) return;
                done=true; deadline.stop();
                if (process.running) process.signal(9);
                if (root.job === worker) {
                    root.job=null;
                    root.completed(context,ok,value);
                }
                Qt.callLater(function(){worker.destroy();});
            }
            function collect() {
                if (done || !sawExit || !sawEof) return;
                var value=null;
                var ok=exitCode===0 && exitStatus===0 && output.length<=32768;
                if (ok) {
                    try { value=JSON.parse(output); ok=value!==null && typeof value==="object" && !Array.isArray(value); }
                    catch (error) { ok=false; }
                }
                finish(ok,ok?value:null);
            }
            Process {
                id: process
                command: worker.argv
                stdout: StdioCollector {
                    waitForEnd: true
                    onStreamFinished: { worker.output=text; worker.sawEof=true; worker.collect(); }
                }
                onExited: function(code,status) {
                    worker.exitCode=code; worker.exitStatus=status; worker.sawExit=true; worker.collect();
                }
            }
            Timer { id: deadline; interval: root.timeoutMs; onTriggered: worker.finish(false,null) }
            Component.onCompleted: { deadline.start(); process.running=true; }
        }
    }
}
