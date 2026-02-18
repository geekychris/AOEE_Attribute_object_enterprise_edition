package com.aoee.client;

/**
 * 64-bit Entity ID with embedded type information.
 * 
 * Layout: [type:16 bits][raw_id:48 bits]
 * 
 * The high 16 bits encode the entity type, and the low 48 bits encode the unique
 * identifier within that type. This allows O(1) type extraction without database lookups.
 * 
 * @see EntityType for predefined type codes
 */
public final class EntityId {
    
    /** Mask for extracting the raw ID (lower 48 bits) */
    private static final long ID_MASK = 0x0000_FFFF_FFFF_FFFFL;
    
    /** Number of bits to shift for type extraction */
    private static final int TYPE_SHIFT = 48;
    
    /** Maximum value for raw ID (2^48 - 1) */
    public static final long MAX_RAW_ID = ID_MASK;
    
    private final long value;
    
    /**
     * Create an EntityId from entity type and raw ID.
     * 
     * @param entityType The entity type
     * @param rawId The raw ID (must be <= MAX_RAW_ID)
     * @throws IllegalArgumentException if rawId exceeds 48 bits
     */
    public EntityId(EntityType entityType, long rawId) {
        if (rawId < 0 || rawId > MAX_RAW_ID) {
            throw new IllegalArgumentException("rawId must be between 0 and " + MAX_RAW_ID);
        }
        this.value = ((long) entityType.getCode() << TYPE_SHIFT) | rawId;
    }
    
    /**
     * Create an EntityId from type code and raw ID.
     * 
     * @param typeCode The entity type code (0-65535)
     * @param rawId The raw ID (must be <= MAX_RAW_ID)
     */
    public EntityId(int typeCode, long rawId) {
        this(EntityType.fromCode(typeCode), rawId);
    }
    
    /**
     * Private constructor for fromRaw.
     */
    private EntityId(long value) {
        this.value = value;
    }
    
    /**
     * Create an EntityId from a raw 64-bit value.
     * Use this when deserializing or receiving IDs from external sources.
     * 
     * @param value The raw 64-bit ID value
     * @return The EntityId
     */
    public static EntityId fromRaw(long value) {
        return new EntityId(value);
    }
    
    /**
     * Get the raw 64-bit value.
     * This is the value that should be stored and transmitted.
     */
    public long getValue() {
        return value;
    }
    
    /**
     * Alias for getValue() for compatibility.
     */
    public long asRaw() {
        return value;
    }
    
    /**
     * Extract the entity type from this ID.
     */
    public EntityType getEntityType() {
        int typeCode = (int) (value >>> TYPE_SHIFT);
        return EntityType.fromCode(typeCode);
    }
    
    /**
     * Extract the type code (0-65535) from this ID.
     */
    public int getTypeCode() {
        return (int) (value >>> TYPE_SHIFT);
    }
    
    /**
     * Extract the raw ID (without type information).
     */
    public long getRawId() {
        return value & ID_MASK;
    }
    
    /**
     * Check if this is a valid ID (non-zero raw_id).
     */
    public boolean isValid() {
        return (value & ID_MASK) != 0;
    }
    
    /**
     * Create a null/invalid EntityId.
     */
    public static EntityId nullId() {
        return new EntityId(0L);
    }
    
    // Type checking convenience methods
    
    public boolean isUser() {
        return getEntityType() == EntityType.USER;
    }
    
    public boolean isPost() {
        return getEntityType() == EntityType.POST;
    }
    
    public boolean isComment() {
        return getEntityType() == EntityType.COMMENT;
    }
    
    public boolean isPhoto() {
        return getEntityType() == EntityType.PHOTO;
    }
    
    public boolean isVideo() {
        return getEntityType() == EntityType.VIDEO;
    }
    
    // Object methods
    
    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (o == null || getClass() != o.getClass()) return false;
        EntityId entityId = (EntityId) o;
        return value == entityId.value;
    }
    
    @Override
    public int hashCode() {
        return Long.hashCode(value);
    }
    
    @Override
    public String toString() {
        return getEntityType().name() + ":" + getRawId();
    }
    
    /**
     * Format as debug string showing all components.
     */
    public String toDebugString() {
        return String.format("EntityId(%s:%d, raw=0x%016X)", 
            getEntityType().name(), getRawId(), value);
    }
}
