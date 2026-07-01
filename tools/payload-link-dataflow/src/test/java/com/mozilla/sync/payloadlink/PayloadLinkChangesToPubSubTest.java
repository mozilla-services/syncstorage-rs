package com.mozilla.sync.payloadlink;

import com.google.cloud.Timestamp;
import java.util.Arrays;
import java.util.Collections;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.DataChangeRecord;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.Mod;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.ModType;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.ValueCaptureType;
import org.junit.Test;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

/**
 * Unit tests for {@link PayloadLinkChangesToPubSub#isPayloadLinkActionable}.
 *
 * <p>The filter is the sole customization this template adds over the
 * upstream Spanner-CS-to-Pub/Sub template; its semantics are load-bearing
 * for the downstream reconciler and Pub/Sub volume budget. Test cases
 * mirror the column-scoped change-stream shapes for {@code payload_link}.
 *
 * <p>Note: {@link DataChangeRecord} constructor signatures have shifted
 * across Beam releases. If a Beam version bump breaks compilation here,
 * adjust the {@link #recordWithMods} helper -- not the assertions.
 */
public class PayloadLinkChangesToPubSubTest {

    private static final String NULL_LINK = "{\"payload_link\":null}";
    private static final String NO_LINK_FIELD = "{}";
    private static final String LINK_A = "{\"payload_link\":\"gs://b/a/c/bso/uuid-a\"}";
    private static final String LINK_B = "{\"payload_link\":\"gs://b/a/c/bso/uuid-b\"}";
    private static final String MALFORMED = "{not-json";

    @Test
    public void insertWithLink_isKept() {
        DataChangeRecord r = recordWithMods(new Mod("{}", NO_LINK_FIELD, LINK_A));
        assertTrue(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void insertWithNullLink_isDropped() {
        DataChangeRecord r = recordWithMods(new Mod("{}", NO_LINK_FIELD, NULL_LINK));
        assertFalse(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void insertWithEmptyMaps_isDropped() {
        DataChangeRecord r = recordWithMods(new Mod("{}", NO_LINK_FIELD, NO_LINK_FIELD));
        assertFalse(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void updateReplacingLink_isKept() {
        DataChangeRecord r = recordWithMods(new Mod("{}", LINK_A, LINK_B));
        assertTrue(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void updateClearingLink_isKept() {
        DataChangeRecord r = recordWithMods(new Mod("{}", LINK_A, NULL_LINK));
        assertTrue(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void updateNullToNull_isDropped() {
        DataChangeRecord r = recordWithMods(new Mod("{}", NULL_LINK, NULL_LINK));
        assertFalse(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void deleteWithLink_isKept() {
        DataChangeRecord r = recordWithMods(new Mod("{}", LINK_A, NO_LINK_FIELD));
        assertTrue(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void deleteWithNullLink_isDropped() {
        DataChangeRecord r = recordWithMods(new Mod("{}", NULL_LINK, NO_LINK_FIELD));
        assertFalse(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void multipleMods_oneActionable_isKept() {
        DataChangeRecord r = recordWithMods(
            new Mod("{}", NULL_LINK, NULL_LINK),
            new Mod("{}", NO_LINK_FIELD, LINK_A),
            new Mod("{}", NULL_LINK, NULL_LINK));
        assertTrue(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void multipleMods_allInert_isDropped() {
        DataChangeRecord r = recordWithMods(
            new Mod("{}", NULL_LINK, NULL_LINK),
            new Mod("{}", NO_LINK_FIELD, NO_LINK_FIELD),
            new Mod("{}", NULL_LINK, NO_LINK_FIELD));
        assertFalse(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void malformedJson_passesThrough() {
        DataChangeRecord r = recordWithMods(new Mod("{}", MALFORMED, NO_LINK_FIELD));
        assertTrue(
            "malformed records must pass through so the reconciler/DLQ surfaces them",
            PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    @Test
    public void emptyJsonStrings_treatedAsNull_dropped() {
        DataChangeRecord r = recordWithMods(new Mod("{}", "", ""));
        assertFalse(PayloadLinkChangesToPubSub.isPayloadLinkActionable(r));
    }

    private static DataChangeRecord recordWithMods(Mod... mods) {
        return new DataChangeRecord(
            "partition-1",                       // partitionToken
            Timestamp.now(),                     // commitTimestamp
            "txn-1",                             // serverTransactionId
            true,                                // isLastRecordInTransactionInPartition
            "seq-1",                             // recordSequence
            "bsos",                              // tableName
            Collections.emptyList(),             // rowType
            Arrays.asList(mods),                 // mods
            ModType.UPDATE,                      // modType
            ValueCaptureType.OLD_AND_NEW_VALUES, // valueCaptureType
            1L,                                  // numberOfRecordsInTransaction
            1L,                                  // numberOfPartitionsInTransaction
            "",                                  // transactionTag
            false,                               // isSystemTransaction
            null                                 // metadata
        );
    }
}
