package com.aoee.client;

/**
 * Entity types supported by AOEE.
 * 
 * The code values are stored in the high 16 bits of EntityId.
 * These must match the Rust EntityType enum in aoee-core/src/id.rs.
 */
public enum EntityType {
    /** User entity (person, account) */
    USER(0),
    /** Post entity (status update, article) */
    POST(1),
    /** Comment entity */
    COMMENT(2),
    /** Photo/Image entity */
    PHOTO(3),
    /** Video entity */
    VIDEO(4),
    /** Group entity */
    GROUP(5),
    /** Page entity (business page, fan page) */
    PAGE(6),
    /** Event entity */
    EVENT(7),
    /** Message entity */
    MESSAGE(8),
    /** Reaction entity (like, love, etc.) */
    REACTION(9),
    /** Tag entity */
    TAG(10),
    /** Location/Place entity */
    LOCATION(11),
    /** Link/URL entity */
    LINK(12),
    /** Album entity */
    ALBUM(13),
    /** Story entity */
    STORY(14),
    /** Custom type 1 (application-defined) */
    CUSTOM1(100),
    /** Custom type 2 (application-defined) */
    CUSTOM2(101),
    /** Custom type 3 (application-defined) */
    CUSTOM3(102),
    /** Unknown/Invalid type */
    UNKNOWN(0xFFFF);
    
    private final int code;
    
    EntityType(int code) {
        this.code = code;
    }
    
    /**
     * Get the numeric type code (0-65535).
     */
    public int getCode() {
        return code;
    }
    
    /**
     * Create EntityType from numeric code.
     * 
     * @param code The type code (0-65535)
     * @return The EntityType, or UNKNOWN if not recognized
     */
    public static EntityType fromCode(int code) {
        return switch (code) {
            case 0 -> USER;
            case 1 -> POST;
            case 2 -> COMMENT;
            case 3 -> PHOTO;
            case 4 -> VIDEO;
            case 5 -> GROUP;
            case 6 -> PAGE;
            case 7 -> EVENT;
            case 8 -> MESSAGE;
            case 9 -> REACTION;
            case 10 -> TAG;
            case 11 -> LOCATION;
            case 12 -> LINK;
            case 13 -> ALBUM;
            case 14 -> STORY;
            case 100 -> CUSTOM1;
            case 101 -> CUSTOM2;
            case 102 -> CUSTOM3;
            default -> UNKNOWN;
        };
    }
    
    /**
     * Parse EntityType from string name (case-insensitive).
     * 
     * @param name The type name
     * @return The EntityType, or UNKNOWN if not recognized
     */
    public static EntityType fromName(String name) {
        if (name == null) return UNKNOWN;
        try {
            return valueOf(name.toUpperCase());
        } catch (IllegalArgumentException e) {
            return UNKNOWN;
        }
    }
}
