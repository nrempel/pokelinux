package org.pokelinux.datatest;

// Authored test infrastructure. Public Data parcelables remain unmodified.
@VintfStability
parcelable CaptureSetup {
    int attachmentAttempts;
    int attachmentsRetained;
    int attachmentCallingPid;
    int attachmentCallingUid;
    int setupAttempts;
    boolean setupPresent;
    int setupCallingPid;
    int setupCallingUid;
    int serial;
    android.hardware.radio.AccessNetwork accessNetwork;
    android.hardware.radio.data.DataProfileInfo dataProfileInfo;
    boolean roamingAllowed;
    android.hardware.radio.data.DataRequestReason reason;
    android.hardware.radio.data.LinkAddress[] addresses;
    String[] dnses;
    int pduSessionId;
    @nullable android.hardware.radio.data.SliceInfo sliceInfo;
    boolean matchAllRuleAllowed;
    int forbiddenCalls;
    int rejectedCalls;
    int responseAttempts;
    int responseSubmissions;
    @nullable android.hardware.radio.RadioResponseInfo lastResponseInfo;
    @nullable android.hardware.radio.data.SetupDataCallResult lastResponse;
    int indicationAttempts;
    int indicationSubmissions;
    android.hardware.radio.data.SetupDataCallResult[] lastDataCallList;
}
