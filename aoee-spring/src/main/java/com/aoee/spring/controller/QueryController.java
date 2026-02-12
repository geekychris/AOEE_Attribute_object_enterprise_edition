package com.aoee.spring.controller;

import com.aoee.client.AoeeClient;
import com.aoee.client.EdgeType;
import com.aoee.client.FofResult;
import com.aoee.spring.model.*;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import java.util.List;

@RestController
@RequestMapping("/api/query")
public class QueryController {

    private final AoeeClient client;

    public QueryController(AoeeClient client) {
        this.client = client;
    }

    @PostMapping("/intersect")
    public ResponseEntity<SetOperationResponse> intersect(@RequestBody SetOperationRequest request) {
        List<Long> result = client.intersect(
                request.src1(),
                EdgeType.fromName(request.edgeType1()),
                request.src2(),
                EdgeType.fromName(request.edgeType2())
        );
        return ResponseEntity.ok(new SetOperationResponse("intersect", result));
    }

    @PostMapping("/union")
    public ResponseEntity<SetOperationResponse> union(@RequestBody SetOperationRequest request) {
        List<Long> result = client.union(
                request.src1(),
                EdgeType.fromName(request.edgeType1()),
                request.src2(),
                EdgeType.fromName(request.edgeType2())
        );
        return ResponseEntity.ok(new SetOperationResponse("union", result));
    }

    @PostMapping("/fof")
    public ResponseEntity<FofResponse> friendOfFriend(@RequestBody FofRequest request) {
        FofResult result = client.friendOfFriend(
                request.source(),
                EdgeType.fromName(request.edgeType()),
                request.fanoutCap() != null ? request.fanoutCap() : 0,
                request.maxResults() != null ? request.maxResults() : 0,
                request.minScore() != null ? request.minScore() : 0,
                request.exclusions() != null ? request.exclusions() : List.of()
        );
        
        List<FofResponse.Candidate> candidates = result.candidates().stream()
                .map(c -> new FofResponse.Candidate(c.id(), c.score()))
                .toList();
        
        return ResponseEntity.ok(new FofResponse(
                request.source(),
                candidates,
                result.truncated(),
                result.elapsedMs()
        ));
    }

    @PostMapping("/mutual-friends")
    public ResponseEntity<SetOperationResponse> mutualFriends(@RequestBody MutualFriendsRequest request) {
        // Mutual friends = intersect(user1.follows, user2.follows)
        List<Long> result = client.intersect(
                request.user1(),
                EdgeType.FOLLOWS,
                request.user2(),
                EdgeType.FOLLOWS
        );
        return ResponseEntity.ok(new SetOperationResponse("mutual_friends", result));
    }
}
