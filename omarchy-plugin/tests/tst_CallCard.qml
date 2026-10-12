import QtQuick
import QtQuick.Controls
import QtTest
import "../qml"

TestCase {
    name: "PhoneCallCard"
    when: windowShown
    width: 412
    height: 600
    visible: true
    Rectangle {
        id: canvas
        anchors.fill: parent
        color: "#101315"
        CallCard { id: card; width: parent.width - 32; anchors.centerIn: parent }
    }
    SignalSpy { id: actions; target: card; signalName: "actionRequested" }
    SignalSpy { id: acknowledgments; target: card; signalName: "acknowledgmentRequested" }
    function init() {
        card.available = true;
        card.authorized = true;
        card.callState = 2;
        card.actions = ["answer", "reject"];
        card.busy = false;
        card.uncertain = false;
        card.pendingAction = "";
        card.recoveryKey="";card.recoveryMessage="";card.canAcknowledge=false;
        card.contextKey = "current-call-revision";
        actions.clear();
        acknowledgments.clear();
        waitForRendering(card);
    }
    function test_ringing_has_accessible_touch_targets() {
        var answer = findChild(card, "answerButton");
        var decline = findChild(card, "declineButton");
        verify(answer.visible && decline.visible);
        verify(answer.width >= 48 && answer.height >= 48);
        verify(decline.width >= 48 && decline.height >= 48);
        mouseClick(answer);
        compare(actions.count, 1);
        compare(actions.signalArguments[0][0], "answer");
    }
    function test_recovery_requires_separate_gesture_and_current_record() {
        verify('recoveryKey' in card, 'Card must bind acknowledgment to a recovery record');
        card.recoveryKey='first-record'; card.recoveryMessage='The earlier action may have taken effect.';
        card.canAcknowledge=true; card.busy=true;
        var button=findChild(card,'acknowledgeButton');
        verify(button.visible && button.enabled);
        verify(button.height>=48);
        mousePress(button);card.recoveryKey='replacement-record';mouseRelease(button);
        compare(actions.count,0);
        compare(acknowledgments.count,0);
        verify(!findChild(card,'answerButton').visible);
        mouseClick(button);compare(actions.count,0);
        compare(acknowledgments.count,1);
        compare(acknowledgments.signalArguments[0][0],'replacement-record');
        card.recoveryKey='';card.recoveryMessage='';card.canAcknowledge=false;card.busy=false;
    }
    function test_stale_pending_and_unknown_do_not_emit() {
        var answer = findChild(card, "answerButton");
        card.available = false;
        mouseClick(answer);
        card.requestAction("answer", card.contextKey);
        card.available = true; card.busy = true;
        card.requestAction("answer", card.contextKey);
        card.busy = false; card.uncertain = true;
        card.requestAction("answer", card.contextKey);
        compare(actions.count, 0);
    }
    function test_wrong_state_and_missing_permission_do_not_emit() {
        card.authorized = false; card.requestAction("answer", card.contextKey);
        card.authorized = true; card.callState = 4; card.requestAction("answer", card.contextKey);
        card.actions = []; card.requestAction("disconnect", card.contextKey);
        compare(actions.count, 0);
    }
    function test_active_exposes_only_end() {
        card.callState = 4; card.actions = ["disconnect"];
        verify(!findChild(card, "answerButton").visible);
        verify(!findChild(card, "declineButton").visible);
        var end = findChild(card, "endButton");
        verify(end.visible && end.enabled);
        mouseClick(end);
        compare(actions.signalArguments[0][0], "disconnect");
    }
    function test_keyboard_and_labels() {
        var answer = findChild(card, "answerButton");
        compare(answer.Accessible.name, "Answer call");
        answer.forceActiveFocus();
        keyClick(Qt.Key_Space);
        compare(actions.count, 1);
    }
    function test_no_automatic_action_on_state_changes() {
        card.callState = 4; card.callState = 2;
        card.available = false; card.available = true;
        card.visible = false; card.visible = true;
        compare(actions.count, 0);
    }
    function test_call_replacement_during_press_cancels_action() {
        verify("contextKey" in card, "Call identity must be bound to the press");
        card.contextKey = "first-call-revision";
        var answer = findChild(card, "answerButton");
        mousePress(answer);
        card.contextKey = "replacement-call-revision";
        mouseRelease(answer);
        compare(actions.count, 0);
    }
    function test_z_render_states() {
        function capture(name) {
            var done = false;
            var saved = false;
            verify(canvas.grabToImage(function(result) {
                saved = result.saveToFile("/tmp/phone-card-" + name + ".png");
                done = true;
            }));
            tryVerify(function() { return done; }, 3000);
            verify(saved);
        }
        capture("incoming");
        card.busy = true; card.pendingAction = "answer";
        wait(50); capture("answering");
        card.busy = false; card.callState = 4; card.actions = ["disconnect"];
        wait(50); capture("active");
        card.uncertain = true;
        wait(50); capture("unknown");
        card.uncertain=false;card.busy=true;card.callState=2;
        card.recoveryKey='record';card.canAcknowledge=true;
        card.recoveryMessage='An earlier action may have taken effect. Acknowledge this uncertainty to refresh the call status.';
        wait(50);capture('recovery');
    }

}
