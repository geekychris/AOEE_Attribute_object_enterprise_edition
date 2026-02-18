package com.aoee.spring.persistence;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.context.annotation.Configuration;

/**
 * Configuration for the persistence service integration.
 */
@Configuration
@ConfigurationProperties(prefix = "aoee.persistence")
public class PersistenceConfig {
    
    /**
     * Whether persistence is enabled at all.
     */
    private boolean enabled = false;
    
    /**
     * URL of the persistence service.
     */
    private String url = "http://localhost:8081";
    
    /**
     * Whether to write-through to persistence on edge modifications.
     */
    private boolean writeThrough = true;
    
    /**
     * Whether to warm the cache from persistence on startup.
     */
    private boolean warmOnStartup = false;

    // Getters and Setters
    public boolean isEnabled() {
        return enabled;
    }

    public void setEnabled(boolean enabled) {
        this.enabled = enabled;
    }

    public String getUrl() {
        return url;
    }

    public void setUrl(String url) {
        this.url = url;
    }

    public boolean isWriteThrough() {
        return writeThrough;
    }

    public void setWriteThrough(boolean writeThrough) {
        this.writeThrough = writeThrough;
    }

    public boolean isWarmOnStartup() {
        return warmOnStartup;
    }

    public void setWarmOnStartup(boolean warmOnStartup) {
        this.warmOnStartup = warmOnStartup;
    }
}
