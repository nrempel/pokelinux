package org.pokelinux.sensortest;
import android.hardware.sensors.ISensors;
import org.pokelinux.sensortest.ISensorClient;
/** Original synthetic producer controls; the sensor API/queues use exact public frozen types. */
@VintfStability
interface ISensorBoundaryControl {
    ISensors sensors();
    int uid();
    long requestSubscription(in ISensorClient client, int claimedUid, int kind);
    int[] eventAbi();
    void produce(int count, long timestamp);
    void produceMalformed();
    int awaitAckTimeout();
    int drainAcks();
    int pendingWake();
    int awaitReadFlag();
    void dropQueues();
    int fdCount();
    void die();
}
