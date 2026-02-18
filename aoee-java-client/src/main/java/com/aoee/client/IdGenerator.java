package com.aoee.client;

import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

/**
 * Thread-safe ID generator using hybrid timestamp + sequence approach.
 * 
 * Generates IDs with the following structure within the 48-bit raw_id:
 * - Upper 32 bits: seconds since AOEE_EPOCH (2020-01-01)
 * - Lower 16 bits: sequence counter (0-65535)
 * 
 * This allows 65,536 IDs per second per generator instance without
 * requiring a central coordinator. The algorithm matches the Rust
 * implementation in aoee-core/src/id.rs.
 * 
 * <p>Example usage:</p>
 * <pre>{@code
 * IdGenerator userGen = new IdGenerator(EntityType.USER);
 * EntityId userId = userGen.nextId();
 * System.out.println(userId);  // "USER:123456789"
 * }</pre>
 * 
 * <p>For server-side ID generation via gRPC, use {@link AoeeClient#generateIds}.</p>
 */
public class IdGenerator {
    
    /**
     * Custom epoch: January 1, 2020 00:00:00 UTC.
     * This gives us ~136 years of range (until ~2156).
     */
    public static final long AOEE_EPOCH = 1577836800L;
    
    /** Bits used for sequence counter in generated IDs */
    private static final int SEQUENCE_BITS = 16;
    
    /** Mask for sequence counter */
    private static final long SEQUENCE_MASK = (1L << SEQUENCE_BITS) - 1;
    
    /** Maximum sequence value */
    private static final int MAX_SEQUENCE = 0xFFFF;
    
    private final EntityType entityType;
    private final AtomicLong lastTimestamp;
    private final AtomicInteger sequence;
    
    /**
     * Create a new ID generator for the specified entity type.
     * 
     * @param entityType The type of entities this generator will create IDs for
     */
    public IdGenerator(EntityType entityType) {
        this.entityType = entityType;
        this.lastTimestamp = new AtomicLong(0);
        this.sequence = new AtomicInteger(0);
    }
    
    /**
     * Get the current timestamp as seconds since AOEE_EPOCH.
     */
    private static long currentTimestamp() {
        return Instant.now().getEpochSecond() - AOEE_EPOCH;
    }
    
    /**
     * Generate the next unique ID.
     * 
     * Thread-safe: multiple threads can call this concurrently.
     * If more than 65,536 IDs are generated in the same second,
     * this will spin-wait until the next second.
     * 
     * @return A new unique EntityId
     */
    public EntityId nextId() {
        while (true) {
            long now = currentTimestamp();
            long last = lastTimestamp.get();
            
            if (now > last) {
                // New second - try to update timestamp and reset sequence
                if (lastTimestamp.compareAndSet(last, now)) {
                    sequence.set(1);  // Store 1 so next fetch returns 1 (we use 0)
                    return makeId(now, 0);
                }
                // Another thread updated - retry
                continue;
            }
            
            // Same second - increment sequence
            int seq = sequence.getAndIncrement();
            if (seq <= MAX_SEQUENCE) {
                return makeId(last, seq);
            }
            
            // Sequence overflow - wait for next second
            Thread.onSpinWait();
        }
    }
    
    /**
     * Generate multiple IDs efficiently.
     * 
     * @param count Number of IDs to generate
     * @return List of unique EntityIds
     */
    public List<EntityId> nextIds(int count) {
        List<EntityId> ids = new ArrayList<>(count);
        for (int i = 0; i < count; i++) {
            ids.add(nextId());
        }
        return ids;
    }
    
    /**
     * Construct an EntityId from timestamp and sequence.
     */
    private EntityId makeId(long timestamp, int seq) {
        long rawId = (timestamp << SEQUENCE_BITS) | (seq & SEQUENCE_MASK);
        return new EntityId(entityType, rawId);
    }
    
    /**
     * Get the entity type this generator produces.
     */
    public EntityType getEntityType() {
        return entityType;
    }
    
    /**
     * Extract timestamp from a generated ID (seconds since AOEE_EPOCH).
     * Only valid for IDs created by an IdGenerator.
     * 
     * @param id The entity ID
     * @return Seconds since AOEE_EPOCH
     */
    public static long extractTimestamp(EntityId id) {
        return id.getRawId() >>> SEQUENCE_BITS;
    }
    
    /**
     * Extract sequence number from a generated ID.
     * Only valid for IDs created by an IdGenerator.
     * 
     * @param id The entity ID
     * @return Sequence number (0-65535)
     */
    public static int extractSequence(EntityId id) {
        return (int) (id.getRawId() & SEQUENCE_MASK);
    }
    
    /**
     * Convert timestamp (seconds since AOEE_EPOCH) to Unix timestamp.
     * 
     * @param aoeeTimestamp Seconds since AOEE_EPOCH
     * @return Unix timestamp (seconds since 1970)
     */
    public static long toUnixTimestamp(long aoeeTimestamp) {
        return AOEE_EPOCH + aoeeTimestamp;
    }
    
    /**
     * Convert Unix timestamp to AOEE timestamp.
     * 
     * @param unixTimestamp Unix timestamp (seconds since 1970)
     * @return Seconds since AOEE_EPOCH
     */
    public static long fromUnixTimestamp(long unixTimestamp) {
        return Math.max(0, unixTimestamp - AOEE_EPOCH);
    }
    
    /**
     * Convert an entity ID to an Instant (for IDs created by IdGenerator).
     * 
     * @param id The entity ID
     * @return The Instant when the ID was created
     */
    public static Instant toInstant(EntityId id) {
        long aoeeTs = extractTimestamp(id);
        return Instant.ofEpochSecond(toUnixTimestamp(aoeeTs));
    }
    
    @Override
    public String toString() {
        return "IdGenerator{" +
                "entityType=" + entityType +
                ", lastTimestamp=" + lastTimestamp.get() +
                ", sequence=" + sequence.get() +
                '}';
    }
}
