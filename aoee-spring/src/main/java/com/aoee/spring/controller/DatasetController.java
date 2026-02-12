package com.aoee.spring.controller;

import com.aoee.spring.model.DatasetLoadResponse;
import com.aoee.spring.model.DatasetParseResult;
import com.aoee.spring.service.DatasetService;
import org.springframework.core.io.ClassPathResource;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Map;

@RestController
@RequestMapping("/api/dataset")
public class DatasetController {

    private final DatasetService datasetService;

    public DatasetController(DatasetService datasetService) {
        this.datasetService = datasetService;
    }

    @PostMapping("/load")
    public ResponseEntity<DatasetLoadResponse> loadDataset(@RequestBody String content) {
        DatasetLoadResponse result = datasetService.loadDataset(content);
        return ResponseEntity.ok(result);
    }

    @PostMapping("/preview")
    public ResponseEntity<DatasetParseResult> previewDataset(@RequestBody String content) {
        DatasetParseResult result = datasetService.parseDataset(content);
        return ResponseEntity.ok(result);
    }

    @GetMapping("/sample")
    public ResponseEntity<Map<String, String>> getSampleDataset() {
        try {
            ClassPathResource resource = new ClassPathResource("sample-dataset.aoee");
            String sample = new String(resource.getInputStream().readAllBytes(), StandardCharsets.UTF_8);
            return ResponseEntity.ok(Map.of(
                    "format", "text",
                    "sample", sample
            ));
        } catch (IOException e) {
            return ResponseEntity.internalServerError().body(Map.of(
                    "error", "Failed to load sample dataset: " + e.getMessage()
            ));
        }
    }
}
