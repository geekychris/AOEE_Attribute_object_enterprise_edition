package com.aoee.client;

/**
 * Edge type constants matching the Rust AOEE implementation.
 * Values MUST match the Rust EdgeType enum in aoee-core/src/types.rs
 */
public final class EdgeType {
    private EdgeType() {} // Prevent instantiation

    // Social connections (0-9)
    public static final int FOLLOWS = 0;
    public static final int FOLLOWED_BY = 1;
    public static final int FRIEND_OF = 2;
    public static final int BLOCKS = 3;
    public static final int BLOCKED_BY = 4;

    // Content interactions (10-19)
    public static final int LIKES = 10;
    public static final int LIKED_BY = 11;
    public static final int COMMENTS_ON = 12;
    public static final int HAS_COMMENT = 13;
    public static final int SHARES = 14;
    public static final int SHARED_BY = 15;

    // Authorship (20-29)
    public static final int AUTHORED = 20;
    public static final int AUTHORED_BY = 21;

    // Group/Page membership (30-39)
    public static final int MEMBER_OF = 30;
    public static final int HAS_MEMBER = 31;
    public static final int ADMINISTERS = 32;
    public static final int ADMINISTERED_BY = 33;

    // Content containment (40-49)
    public static final int CONTAINS = 40;
    public static final int CONTAINED_IN = 41;

    // Tagging (50-59)
    public static final int TAGGED_IN = 50;
    public static final int HAS_TAG = 51;

    // Mentions (60-69)
    public static final int MENTIONS = 60;
    public static final int MENTIONED_BY = 61;

    // Custom (100+)
    public static final int CUSTOM1 = 100;
    public static final int CUSTOM2 = 101;
    public static final int CUSTOM3 = 102;
    public static final int CUSTOM4 = 103;
    public static final int CUSTOM5 = 104;

    /**
     * Get the reverse edge type, if applicable.
     */
    public static int reverse(int edgeType) {
        return switch (edgeType) {
            case FOLLOWS -> FOLLOWED_BY;
            case FOLLOWED_BY -> FOLLOWS;
            case FRIEND_OF -> FRIEND_OF; // Symmetric
            case BLOCKS -> BLOCKED_BY;
            case BLOCKED_BY -> BLOCKS;
            case LIKES -> LIKED_BY;
            case LIKED_BY -> LIKES;
            case COMMENTS_ON -> HAS_COMMENT;
            case HAS_COMMENT -> COMMENTS_ON;
            case SHARES -> SHARED_BY;
            case SHARED_BY -> SHARES;
            case AUTHORED -> AUTHORED_BY;
            case AUTHORED_BY -> AUTHORED;
            case MEMBER_OF -> HAS_MEMBER;
            case HAS_MEMBER -> MEMBER_OF;
            case ADMINISTERS -> ADMINISTERED_BY;
            case ADMINISTERED_BY -> ADMINISTERS;
            case CONTAINS -> CONTAINED_IN;
            case CONTAINED_IN -> CONTAINS;
            case TAGGED_IN -> HAS_TAG;
            case HAS_TAG -> TAGGED_IN;
            case MENTIONS -> MENTIONED_BY;
            case MENTIONED_BY -> MENTIONS;
            default -> -1;
        };
    }

    /**
     * Get the name of an edge type.
     */
    public static String name(int edgeType) {
        return switch (edgeType) {
            case FOLLOWS -> "FOLLOWS";
            case FOLLOWED_BY -> "FOLLOWED_BY";
            case FRIEND_OF -> "FRIEND_OF";
            case BLOCKS -> "BLOCKS";
            case BLOCKED_BY -> "BLOCKED_BY";
            case LIKES -> "LIKES";
            case LIKED_BY -> "LIKED_BY";
            case COMMENTS_ON -> "COMMENTS_ON";
            case HAS_COMMENT -> "HAS_COMMENT";
            case SHARES -> "SHARES";
            case SHARED_BY -> "SHARED_BY";
            case AUTHORED -> "AUTHORED";
            case AUTHORED_BY -> "AUTHORED_BY";
            case MEMBER_OF -> "MEMBER_OF";
            case HAS_MEMBER -> "HAS_MEMBER";
            case ADMINISTERS -> "ADMINISTERS";
            case ADMINISTERED_BY -> "ADMINISTERED_BY";
            case CONTAINS -> "CONTAINS";
            case CONTAINED_IN -> "CONTAINED_IN";
            case TAGGED_IN -> "TAGGED_IN";
            case HAS_TAG -> "HAS_TAG";
            case MENTIONS -> "MENTIONS";
            case MENTIONED_BY -> "MENTIONED_BY";
            case CUSTOM1 -> "CUSTOM1";
            case CUSTOM2 -> "CUSTOM2";
            case CUSTOM3 -> "CUSTOM3";
            case CUSTOM4 -> "CUSTOM4";
            case CUSTOM5 -> "CUSTOM5";
            default -> "UNKNOWN(" + edgeType + ")";
        };
    }

    /**
     * Parse edge type from name.
     */
    public static int fromName(String name) {
        return switch (name.toUpperCase()) {
            case "FOLLOWS" -> FOLLOWS;
            case "FOLLOWED_BY" -> FOLLOWED_BY;
            case "FRIEND_OF", "FRIEND" -> FRIEND_OF;
            case "BLOCKS" -> BLOCKS;
            case "BLOCKED_BY" -> BLOCKED_BY;
            case "LIKES" -> LIKES;
            case "LIKED_BY" -> LIKED_BY;
            case "COMMENTS_ON" -> COMMENTS_ON;
            case "HAS_COMMENT" -> HAS_COMMENT;
            case "SHARES" -> SHARES;
            case "SHARED_BY" -> SHARED_BY;
            case "AUTHORED" -> AUTHORED;
            case "AUTHORED_BY" -> AUTHORED_BY;
            case "MEMBER_OF", "MEMBER" -> MEMBER_OF;
            case "HAS_MEMBER" -> HAS_MEMBER;
            case "ADMINISTERS" -> ADMINISTERS;
            case "ADMINISTERED_BY" -> ADMINISTERED_BY;
            case "CONTAINS" -> CONTAINS;
            case "CONTAINED_IN" -> CONTAINED_IN;
            case "TAGGED_IN", "TAGGED" -> TAGGED_IN;
            case "HAS_TAG" -> HAS_TAG;
            case "MENTIONS" -> MENTIONS;
            case "MENTIONED_BY" -> MENTIONED_BY;
            case "CUSTOM1" -> CUSTOM1;
            case "CUSTOM2" -> CUSTOM2;
            case "CUSTOM3" -> CUSTOM3;
            case "CUSTOM4" -> CUSTOM4;
            case "CUSTOM5" -> CUSTOM5;
            default -> throw new IllegalArgumentException("Unknown edge type: " + name);
        };
    }
}
