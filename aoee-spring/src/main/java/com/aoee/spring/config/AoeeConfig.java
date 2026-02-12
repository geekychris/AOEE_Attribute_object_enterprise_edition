package com.aoee.spring.config;

import com.aoee.client.AoeeClient;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

@Configuration
public class AoeeConfig {

    @Value("${aoee.host:localhost}")
    private String host;

    @Value("${aoee.port:50051}")
    private int port;

    @Bean
    public AoeeClient aoeeClient() {
        return new AoeeClient(host, port);
    }
}
