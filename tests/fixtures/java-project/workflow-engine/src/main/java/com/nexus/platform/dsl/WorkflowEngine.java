package com.nexus.platform.dsl;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class WorkflowEngine {

    private final Map<String, CompiledWorkflow> workflows = new HashMap<>();
    private final RuleEvaluator ruleEvaluator;
    private final DslParser parser;

    public WorkflowEngine() {
        this.ruleEvaluator = new RuleEvaluator();
        this.parser = new DslParser();
    }

    public void loadWorkflow(String xmlContent) {
        CompiledWorkflow workflow = parser.parseWorkflow(xmlContent);
        workflows.put(workflow.name(), workflow);
    }

    public WorkflowResult execute(String workflowName, TransactionContext context) {
        CompiledWorkflow workflow = workflows.get(workflowName);
        if (workflow == null) {
            throw new IllegalArgumentException("Unknown workflow: " + workflowName);
        }

        WorkflowResult.Builder result = new WorkflowResult.Builder(workflowName);

        // Execute stages in dependency order
        for (Stage stage : workflow.orderedStages()) {
            // Check if prerequisite stage passed
            if (stage.requires() != null) {
                StageResult prereq = result.getStageResult(stage.requires());
                if (prereq == null || prereq.outcome() == StageOutcome.REJECTED) {
                    result.addStageResult(stage.name(),
                            new StageResult(StageOutcome.SKIPPED, List.of()));
                    continue;
                }
            }

            // Evaluate all rules in the stage
            List<RuleResult> ruleResults = stage.rules().stream()
                    .map(rule -> ruleEvaluator.evaluate(rule, context))
                    .toList();

            // Check for gate (manual review point)
            if (stage.gate() != null) {
                result.addStageResult(stage.name(),
                        new StageResult(StageOutcome.PENDING_REVIEW, ruleResults));
                result.setPendingGate(stage.gate());
                break; // Pause execution at gate
            }

            // Determine stage outcome from rule results
            StageOutcome outcome = determineOutcome(ruleResults);
            result.addStageResult(stage.name(), new StageResult(outcome, ruleResults));

            if (outcome == StageOutcome.REJECTED) {
                break; // Short-circuit on rejection
            }
        }

        return result.build();
    }

    public List<String> listWorkflows() {
        return List.copyOf(workflows.keySet());
    }

    private StageOutcome determineOutcome(List<RuleResult> results) {
        boolean hasReject = results.stream()
                .anyMatch(r -> r.action() == RuleAction.REJECT);
        if (hasReject) return StageOutcome.REJECTED;

        boolean hasFlag = results.stream()
                .anyMatch(r -> r.action() == RuleAction.FLAG);
        if (hasFlag) return StageOutcome.FLAGGED;

        return StageOutcome.PASSED;
    }

    // --- Inner types ---

    public record CompiledWorkflow(String name, String version, String triggerEvent,
                                   List<Stage> orderedStages) {}

    public record Stage(String name, String requires, List<Rule> rules, Gate gate) {}

    public record Rule(String id, String field, String operator, String value,
                       RuleAction action, String reason) {}

    public record Gate(String type, String assignee, String timeoutDuration,
                       String timeoutAction) {}

    public record TransactionContext(Map<String, Object> fields) {
        public Object getField(String name) {
            return fields.get(name);
        }
    }

    public record RuleResult(String ruleId, RuleAction action, String reason) {}

    public record StageResult(StageOutcome outcome, List<RuleResult> ruleResults) {}

    public record WorkflowResult(String workflowName, Map<String, StageResult> stages,
                                  Gate pendingGate) {
        static class Builder {
            private final String name;
            private final Map<String, StageResult> stages = new java.util.LinkedHashMap<>();
            private Gate pendingGate;

            Builder(String name) { this.name = name; }

            void addStageResult(String stageName, StageResult result) {
                stages.put(stageName, result);
            }

            StageResult getStageResult(String stageName) { return stages.get(stageName); }

            void setPendingGate(Gate gate) { this.pendingGate = gate; }

            WorkflowResult build() {
                return new WorkflowResult(name, Map.copyOf(stages), pendingGate);
            }
        }
    }

    public enum RuleAction { PASS, FLAG, REJECT, ROUTE }

    public enum StageOutcome { PASSED, FLAGGED, REJECTED, PENDING_REVIEW, SKIPPED }
}
