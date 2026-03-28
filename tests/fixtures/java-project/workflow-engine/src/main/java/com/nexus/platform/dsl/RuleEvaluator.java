package com.nexus.platform.dsl;

import com.nexus.platform.dsl.WorkflowEngine.Rule;
import com.nexus.platform.dsl.WorkflowEngine.RuleAction;
import com.nexus.platform.dsl.WorkflowEngine.RuleResult;
import com.nexus.platform.dsl.WorkflowEngine.TransactionContext;

import java.math.BigDecimal;
import java.util.Arrays;
import java.util.regex.Pattern;

public class RuleEvaluator {

    public RuleResult evaluate(Rule rule, TransactionContext context) {
        Object fieldValue = context.getField(rule.field());
        if (fieldValue == null) {
            // Missing field — rule does not match, pass through
            return new RuleResult(rule.id(), RuleAction.PASS, null);
        }

        boolean matches = evaluateCondition(fieldValue, rule.operator(), rule.value());

        if (matches) {
            return new RuleResult(rule.id(), rule.action(), rule.reason());
        } else {
            return new RuleResult(rule.id(), RuleAction.PASS, null);
        }
    }

    private boolean evaluateCondition(Object fieldValue, String operator, String expected) {
        return switch (operator) {
            case "eq" -> fieldValue.toString().equals(expected);
            case "neq" -> !fieldValue.toString().equals(expected);
            case "gt" -> compareNumeric(fieldValue, expected) > 0;
            case "lt" -> compareNumeric(fieldValue, expected) < 0;
            case "gte" -> compareNumeric(fieldValue, expected) >= 0;
            case "lte" -> compareNumeric(fieldValue, expected) <= 0;
            case "in" -> {
                String[] allowed = expected.split(",");
                yield Arrays.asList(allowed).contains(fieldValue.toString());
            }
            case "not_in" -> {
                String[] disallowed = expected.split(",");
                yield !Arrays.asList(disallowed).contains(fieldValue.toString());
            }
            case "regex" -> Pattern.matches(expected, fieldValue.toString());
            case "contains" -> fieldValue.toString().contains(expected);
            case "starts_with" -> fieldValue.toString().startsWith(expected);
            default -> throw new IllegalArgumentException("Unknown operator: " + operator);
        };
    }

    private int compareNumeric(Object fieldValue, String expected) {
        BigDecimal actual = new BigDecimal(fieldValue.toString());
        BigDecimal exp = new BigDecimal(expected);
        return actual.compareTo(exp);
    }
}
