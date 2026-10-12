import QtQuick
import QtTest
import "../qml"

TestCase {
    id: testRoot
    name: "PhoneCallController"
    when: windowShown
    visible: true
    width: 412
    height: 600
    Rectangle { anchors.fill: parent; color: "#101315" }
    CallCard {
        id: card
        width: 380
        anchors.centerIn: parent
        available: controller.available
        authorized: controller.authorized
        callState: controller.callState
        actions: controller.actions
        busy: controller.busy
        uncertain: controller.uncertain
        pendingAction: controller.pendingAction
        contextKey: controller.contextKey
        onActionRequested: function(action, context) {
            controller.requestAction(action,context,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        }
    }
    CallController { id: controller }
    SignalSpy { id: sent; target: controller; signalName: "dispatchRequested" }
    function snapshot(sequence, revision) {
        return {available: true, snapshot: {epoch: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", sequence: sequence,
            controls: {fresh: true, value: {generation: "cccccccccccccccccccccccccccccccc", revision: revision,
                controls_allowed: true, availability: "observed", calls: [{id: 7, state: 2, actions: ["answer", "reject"]}]}},
            dial_status: {fresh: false, value: null}}};
    }
    function init() { controller.reset(); sent.clear(); }
    function capture(name) {
        var done=false; var saved=false;
        verify(testRoot.grabToImage(function(image) {
            saved=image.saveToFile("/tmp/phone-card-controller-"+name+".png"); done=true;
        }));
        tryVerify(function(){return done;},3000); verify(saved);
    }
    function test_real_card_press_and_pending_presentation() {
        controller.acceptSnapshot(snapshot(1,3),controller.barrier);
        waitForRendering(card);
        var button=findChild(card,"answerButton");
        mousePress(button);
        controller.acceptSnapshot(snapshot(2,4),controller.barrier);
        mouseRelease(button);
        compare(sent.count,0);
        mouseClick(button);
        compare(sent.count,1);
        compare(sent.signalArguments[0][0].command.expected_revision,4);
        compare(card.title,"Answering…");
        capture("answering");
        verify(!button.enabled);
        controller.acceptOperation({available:false,epoch:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",id:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"});
        compare(card.title,"Call status unknown");
        capture("unknown");
        mouseClick(button);
        compare(sent.count,1);
    }
    function test_exact_observation_and_single_submission() {
        verify(controller.acceptSnapshot(snapshot(1,3),controller.barrier));
        var key=controller.contextKey;
        verify(controller.requestAction("answer",key,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        compare(sent.count,1);
        var r=sent.signalArguments[0][0];
        compare(r.epoch,"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        compare(r.command.call_id,7); compare(r.command.expected_revision,3); compare(r.command.expected_state,2);
        verify(!controller.requestAction("answer",key,"dddddddddddddddddddddddddddddddd"));
        compare(sent.count,1);
    }
    function test_stale_press_and_old_poll_are_rejected() {
        controller.acceptSnapshot(snapshot(1,3),controller.barrier);
        var key=controller.contextKey;
        controller.acceptSnapshot(snapshot(2,4),controller.barrier);
        verify(!controller.requestAction("answer",key,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        var oldTicket=controller.barrier;
        verify(controller.requestAction("answer",controller.contextKey,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        verify(!controller.acceptSnapshot(snapshot(3,5),oldTicket));
        compare(sent.count,1);
    }
    function test_out_of_order_poll_cannot_replace_current_call() {
        controller.acceptSnapshot(snapshot(2,4),controller.barrier);
        verify(!controller.acceptSnapshot(snapshot(1,3),controller.barrier));
        compare(controller.revision,4);
    }
    function test_unknown_and_absent_receipts_do_not_repeat_command() {
        controller.acceptSnapshot(snapshot(1,3),controller.barrier);
        controller.requestAction("answer",controller.contextKey,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        verify(controller.acceptOperation({available:false,epoch:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",id:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}));
        verify(controller.uncertain);
        controller.acceptSnapshot(snapshot(2,4),controller.barrier);
        verify(!controller.requestAction("answer",controller.contextKey,"dddddddddddddddddddddddddddddddd"));
        controller.acceptOperation({available:true,epoch:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",id:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",operation:null});
        verify(controller.uncertain); compare(sent.count,1);
    }
    function test_result_identity_and_fresh_observation_barrier() {
        controller.acceptSnapshot(snapshot(1,3),controller.barrier);
        controller.requestAction("answer",controller.contextKey,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        var ticket=controller.barrier;
        var response={available:true,epoch:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",id:"dddddddddddddddddddddddddddddddd",operation:{state:"finished",result:{outcome:"submitted",reason:"accepted"}}};
        verify(!controller.acceptOperation(response)); verify(controller.busy);
        response.id="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        verify(controller.acceptOperation(response)); verify(!controller.busy); verify(!controller.available);
        verify(!controller.acceptSnapshot(snapshot(2,4),ticket));
        verify(controller.acceptSnapshot(snapshot(3,4),controller.barrier));
        verify(controller.available); compare(sent.count,1);
    }
    function test_expired_or_disabled_state_cannot_submit() {
        controller.acceptSnapshot(snapshot(1,3),controller.barrier); var key=controller.contextKey;
        controller.expire(); verify(!controller.requestAction("answer",key,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        var value=snapshot(2,3); value.snapshot.controls.value.controls_allowed=false;
        controller.acceptSnapshot(value,controller.barrier);
        verify(!controller.requestAction("answer",controller.contextKey,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        compare(sent.count,0);
    }
}
