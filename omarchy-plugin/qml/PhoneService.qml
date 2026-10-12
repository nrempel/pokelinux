import QtQuick

// This object belongs to the shell service, never to a visible panel.
// Session claims fence old helpers. Only explicit requestAction dispatches calls.
Item {
    id: root
    property var shell: null
    property var clientCommand: ["/usr/bin/omarchy-phone-local"]
    property alias controller: state
    property string preparingAction: ""
    property string error: ""
    property string sessionEpoch: ""
    property string sessionToken: ""
    property int recoveryVersion: 0
    property bool sessionReady: false
    property bool recoveryKnown: false
    property var recoveryItems: []
    readonly property bool busy: identity.active || state.busy || command.active
        || !sessionReady || !recoveryKnown || recoveryItems.length>0 || acknowledgment.active
    readonly property bool canAcknowledge: sessionReady && recoveryKnown && recoveryItems.length>0
        && recoveryItems[0].state!=="pending" && !command.active && !identity.active && !acknowledgment.active
    readonly property string recoveryKey: !sessionReady || !recoveryKnown ? "checking"
        : recoveryItems.length ? JSON.stringify([sessionEpoch,sessionToken,recoveryItems[0].epoch,recoveryItems[0].id]) : ""
    readonly property string recoveryMessage: !sessionReady || !recoveryKnown ? "Checking earlier phone actions…"
        : recoveryItems.length ? (recoveryItems[0].state==="pending" ? "Waiting for an earlier action to finish."
            : "An earlier action may have taken effect. Acknowledge this uncertainty to refresh the call status.") : ""
    readonly property string pendingAction: state.pending ? state.pendingAction : preparingAction
    function unavailable(context) { return {available:false,epoch:context.epoch,id:context.id}; }
    function invalidateSession() {
        recoveryVersion++; sessionReady=false; recoveryKnown=false; sessionToken=""; state.expire();
    }
    function acceptRecovery(page) {
        var hadBlockers=recoveryItems.length>0 || !recoveryKnown;
        recoveryItems=state.clone(page.items); recoveryKnown=true;
        if (hadBlockers && recoveryItems.length===0) state.expire();
    }
    function poll() {
        if (!snapshot.active) snapshot.start(clientCommand.concat(["snapshot"]),state.barrier);
        if (!sessionReady) {
            if (!session.active && !command.active && !identity.active)
                session.start(clientCommand.concat(["session"]),recoveryVersion);
        } else if (!recovery.active && !acknowledgment.active && (!recoveryKnown || recoveryItems.length>0)) {
            recovery.start(clientCommand.concat(["recovery"]),recoveryVersion);
        }
    }
    function acknowledge(observedKey) {
        if (!canAcknowledge || observedKey!==recoveryKey) return false;
        var item=recoveryItems[0];
        recoveryVersion++; state.expire();
        return acknowledgment.start(clientCommand.concat(["acknowledge","--epoch",sessionEpoch,
            "--session",sessionToken,"--record-epoch",item.epoch,"--record-id",item.id]),recoveryVersion);
    }
    function requestAction(action, context) {
        if (busy || !state.available || !state.authorized || context !== state.contextKey
            || state.actions.indexOf(action)<0) return false;
        error=""; preparingAction=action;
        return identity.start(["/usr/bin/python3","-c",
            "import json,uuid;print(json.dumps({'id':uuid.uuid4().hex}))"],
            {action:action,context:context,session:sessionToken,version:recoveryVersion});
    }
    function acceptResult(context,ok,value) {
        if (!state.acceptOperation(ok?value:unavailable(context)))
            state.acceptOperation(unavailable(context));
        if (state.uncertain && sessionReady) invalidateSession();
    }
    CallController {
        id: state
        onDispatchRequested: function(request) {
            var c=request.command;
            var arguments=root.clientCommand.concat(["control","--epoch",request.epoch,"--id",request.id,
                "--session",root.sessionToken,
                "--generation",c.generation,"--call-id",String(c.call_id),"--revision",String(c.expected_revision),
                "--state",String(c.expected_state),"--action",c.action]);
            if (!command.start(arguments,request)) state.acceptOperation(root.unavailable(request));
        }
    }
    JsonCommand {
        id: identity
        onCompleted: function(context,ok,value) {
            root.preparingAction="";
            if (!ok || !value || context.session!==root.sessionToken || context.version!==root.recoveryVersion
                || !root.sessionReady || !root.recoveryKnown || root.recoveryItems.length>0
                || !state.requestAction(context.action,context.context,value.id))
                root.error="Call state changed or the command could not be prepared.";
        }
    }
    JsonCommand {
        id: snapshot
        onCompleted: function(context,ok,value) {
            if (state.acceptSnapshot(ok?value:{available:false},context)) {
                expiry.restart();
                if (root.sessionReady && state.lastEpoch!==root.sessionEpoch) root.invalidateSession();
            }
        }
    }
    JsonCommand {
        id: session
        onCompleted: function(context,ok,value) {
            if (context!==root.recoveryVersion || !ok || !value || value.available!==true || !value.session) return;
            var descriptor=state.clone(value.session);
            root.sessionEpoch=descriptor.epoch; root.sessionToken=descriptor.session;
            // Successful daemon fencing makes old unadmitted requests ineligible.
            // Admitted uncertainty is now represented by the durable recovery list.
            state.pending=null; state.expire();
            root.acceptRecovery(descriptor.recovery); root.sessionReady=true;
        }
    }
    JsonCommand {
        id: recovery
        onCompleted: function(context,ok,value) {
            if (context!==root.recoveryVersion || !root.sessionReady) return;
            if (!ok || !value || value.available!==true || !value.recovery) { root.recoveryKnown=false; return; }
            if (value.recovery.epoch!==root.sessionEpoch) { root.invalidateSession(); return; }
            root.acceptRecovery(state.clone(value.recovery));
        }
    }
    JsonCommand {
        id: acknowledgment
        onCompleted: function(context,ok,value) {
            if (context!==root.recoveryVersion) return;
            root.recoveryKnown=false; state.expire();
            if (!ok || !value || value.available!==true) root.error="Acknowledgment could not be confirmed.";
        }
    }
    JsonCommand {
        id: command
        onCompleted: function(context,ok,value) { root.acceptResult(context,ok,value); }
    }
    JsonCommand {
        id: receipt
        onCompleted: function(context,ok,value) { root.acceptResult(context,ok,value); }
    }
    Timer { id: expiry; interval: 3000; onTriggered: state.expire() }
    Timer {
        interval: 500; running: true; repeat: true; triggeredOnStart: true
        onTriggered: root.poll()
    }
    Timer {
        interval: 500; running: true; repeat: true
        onTriggered: {
            if (!state.pending || command.active || receipt.active) return;
            var context={epoch:state.pending.epoch,id:state.pending.id};
            receipt.start(root.clientCommand.concat(["result","--epoch",context.epoch,"--id",context.id]),context);
        }
    }
}
