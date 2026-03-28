package com.nexus.platform.dsl;

import com.nexus.platform.dsl.WorkflowEngine.*;

import javax.xml.parsers.DocumentBuilder;
import javax.xml.parsers.DocumentBuilderFactory;
import org.w3c.dom.Document;
import org.w3c.dom.Element;
import org.w3c.dom.NodeList;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

public class DslParser {

    public CompiledWorkflow parseWorkflow(String xmlContent) {
        try {
            DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
            // Disable external entities for security
            factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
            DocumentBuilder builder = factory.newDocumentBuilder();
            Document doc = builder.parse(
                    new ByteArrayInputStream(xmlContent.getBytes(StandardCharsets.UTF_8)));

            Element root = doc.getDocumentElement();
            String name = root.getAttribute("name");
            String version = root.getAttribute("version");

            // Parse trigger
            String triggerEvent = null;
            NodeList triggers = root.getElementsByTagName("trigger");
            if (triggers.getLength() > 0) {
                triggerEvent = ((Element) triggers.item(0)).getAttribute("event");
            }

            // Parse stages
            List<Stage> stages = parseStages(root);

            return new CompiledWorkflow(name, version, triggerEvent, stages);
        } catch (Exception e) {
            throw new RuntimeException("Failed to parse NWDL workflow: " + e.getMessage(), e);
        }
    }

    private List<Stage> parseStages(Element root) {
        List<Stage> stages = new ArrayList<>();
        NodeList stageNodes = root.getElementsByTagName("stage");

        for (int i = 0; i < stageNodes.getLength(); i++) {
            Element stageEl = (Element) stageNodes.item(i);
            String name = stageEl.getAttribute("name");
            String requires = stageEl.hasAttribute("requires")
                    ? stageEl.getAttribute("requires") : null;

            List<Rule> rules = parseRules(stageEl);
            Gate gate = parseGate(stageEl);

            stages.add(new Stage(name, requires, rules, gate));
        }

        return stages;
    }

    private List<Rule> parseRules(Element stageEl) {
        List<Rule> rules = new ArrayList<>();
        NodeList ruleNodes = stageEl.getElementsByTagName("rule");

        for (int i = 0; i < ruleNodes.getLength(); i++) {
            Element ruleEl = (Element) ruleNodes.item(i);
            String id = ruleEl.getAttribute("id");

            // Parse condition
            Element condEl = (Element) ruleEl.getElementsByTagName("condition").item(0);
            String field = condEl.getAttribute("field");
            String operator = condEl.getAttribute("operator");
            String value = condEl.getAttribute("value");

            // Parse action
            Element actEl = (Element) ruleEl.getElementsByTagName("action").item(0);
            RuleAction action = RuleAction.valueOf(actEl.getAttribute("type").toUpperCase());
            String reason = actEl.hasAttribute("reason") ? actEl.getAttribute("reason") : null;

            rules.add(new Rule(id, field, operator, value, action, reason));
        }

        return rules;
    }

    private Gate parseGate(Element stageEl) {
        NodeList gateNodes = stageEl.getElementsByTagName("gate");
        if (gateNodes.getLength() == 0) return null;

        Element gateEl = (Element) gateNodes.item(0);
        String type = gateEl.getAttribute("type");
        String assignee = gateEl.getAttribute("assignee");

        // Parse optional timeout
        String timeoutDuration = null;
        String timeoutAction = null;
        NodeList timeoutNodes = stageEl.getElementsByTagName("timeout");
        if (timeoutNodes.getLength() > 0) {
            Element timeoutEl = (Element) timeoutNodes.item(0);
            timeoutDuration = timeoutEl.getAttribute("duration");
            timeoutAction = timeoutEl.getAttribute("action");
        }

        return new Gate(type, assignee, timeoutDuration, timeoutAction);
    }
}
