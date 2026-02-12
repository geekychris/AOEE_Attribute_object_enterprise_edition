package com.aoee.client;

/**
 * Reaction types for LIKES edges.
 * These values are stored as the metadata byte when edgeType = LIKES (10).
 */
public enum ReactionType {
    LIKE(0),
    LOVE(1),
    HAHA(2),
    WOW(3),
    SAD(4),
    ANGRY(5);

    private final int value;

    ReactionType(int value) {
        this.value = value;
    }

    public int getValue() {
        return value;
    }

    public static ReactionType fromValue(int value) {
        return switch (value) {
            case 1 -> LOVE;
            case 2 -> HAHA;
            case 3 -> WOW;
            case 4 -> SAD;
            case 5 -> ANGRY;
            default -> LIKE;
        };
    }

    /**
     * Parse reaction type from name (case-insensitive).
     * @param name Reaction name (e.g., "like", "love", "haha")
     * @return The integer value for the reaction type
     * @throws IllegalArgumentException if name is not recognized
     */
    public static int fromName(String name) {
        return switch (name.toLowerCase()) {
            case "like" -> LIKE.value;
            case "love" -> LOVE.value;
            case "haha" -> HAHA.value;
            case "wow" -> WOW.value;
            case "sad" -> SAD.value;
            case "angry" -> ANGRY.value;
            default -> throw new IllegalArgumentException("Unknown reaction type: " + name);
        };
    }
}
