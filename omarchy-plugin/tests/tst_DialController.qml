import QtQuick
import QtTest
import "../qml"

TestCase {
    name: "PhoneDialController"
    CallController { id: controller }
    SignalSpy { id: sent; target: controller; signalName: "dispatchRequested" }
    readonly property string epoch: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    readonly property string generation: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    readonly property string identity: "cccccccccccccccccccccccccccccccc"
    readonly property string offer: "dddddddddddddddddddddddddddddddd"
    function observe(sequence, changes) {
        var value={generation:generation,can_dial:true,offer:offer,pending:false,reason:"ready"};
        for (var key in changes) value[key]=changes[key];
        return controller.acceptSnapshot({available:true,snapshot:{epoch:epoch,sequence:sequence,
            controls:{fresh:false,value:null},dial_status:{fresh:true,value:value}}},controller.barrier);
    }
    function init() { controller.reset(); sent.clear(); }
    function receipt(outcome) {
        return {available:true,epoch:epoch,id:identity,operation:{state:"finished",
            result:{outcome:outcome,reason:"returned"}}};
    }
    function test_original_offer_survives_poll_refresh() {
        verify(observe(1,{}));
        var context=controller.dialContext;
        verify(observe(2,{offer:"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"}));
        verify(controller.requestDial("+15550100",context,identity));
        compare(sent.count,1);
        compare(sent.signalArguments[0][0].op,"dial");
        compare(sent.signalArguments[0][0].command.offer,offer);
        compare(sent.signalArguments[0][0].command.number,"+15550100");
        verify(!controller.requestDial("5550101",context,identity));
        verify(!controller.acceptOperation(receipt("submitted")));
        verify(controller.acceptOperation(receipt("invocation_attempted")));
        verify(!controller.busy); verify(!controller.dialAvailable);
    }
    function test_changed_generation_and_expiry_reject_intent() {
        observe(1,{}); var context=controller.dialContext;
        observe(2,{generation:"ffffffffffffffffffffffffffffffff"});
        verify(!controller.requestDial("5550100",context,identity));
        context=controller.dialContext; controller.expire();
        verify(!controller.requestDial("5550100",context,identity));
        compare(sent.count,0);
    }
    function test_invalid_number_and_disabled_lane() {
        observe(1,{});
        verify(!controller.requestDial("555;0100",controller.dialContext,identity));
        observe(2,{can_dial:false,offer:"",reason:"disabled"});
        verify(!controller.requestDial("5550100",controller.dialContext,identity));
        compare(sent.count,0);
    }
    function test_unknown_dial_blocks_new_requests() {
        observe(1,{}); verify(controller.requestDial("5550100",controller.dialContext,identity));
        verify(controller.acceptOperation(receipt("unknown")));
        observe(2,{}); verify(controller.uncertain);
        verify(!controller.requestDial("5550101",controller.dialContext,identity));
        compare(sent.count,1);
        verify(controller.acceptOperation(receipt("not_invoked")));
        verify(!controller.busy);
    }
}
