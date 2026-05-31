import com.sun.net.httpserver.HttpServer;
import com.sun.net.httpserver.HttpHandler;
import com.sun.net.httpserver.HttpExchange;

import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;

public class TranslatorServer {

    private static final int SERVER_PORT = 7779;
    private static final int LLAMA_PORT = 7780;
    private static final String LLAMA_HOST = "127.0.0.1";

    private Process llamaProcess;
    private HttpServer httpServer;
    private String modelPath;
    private String baseDir;
    private String llamaBinary;
    private Map<String, String> config;

    public static void main(String[] args) throws Exception {
        TranslatorServer app = new TranslatorServer();
        app.baseDir = getBaseDir();
        app.config = loadConfig(app.baseDir);

        app.llamaBinary = detectLlamaBinary(app.baseDir, app.config);
        if (app.llamaBinary == null) {
            System.err.println("[错误] 未找到 llama-server，请安装 llama.cpp");
            System.exit(1);
        }

        app.modelPath = detectModel(app.baseDir, app.config.get("model"));
        if (app.modelPath == null) {
            System.err.println("[错误] models/ 目录中没有找到模型文件");
            System.exit(1);
        }

        System.out.println("[信息] 配置文件: " + (app.config.isEmpty() ? "未找到，使用默认值" : "已加载"));
        System.out.println("[信息] 模型: " + app.modelPath);
        System.out.println("[信息] llama-server: " + app.llamaBinary);

        Runtime.getRuntime().addShutdownHook(new Thread(app::shutdown));

        app.startLlamaServer();
        app.startHttpServer();

        System.out.println("[信息] 服务已启动: http://localhost:" + SERVER_PORT);
    }

    private static String getBaseDir() {
        try {
            String path = TranslatorServer.class.getProtectionDomain()
                    .getCodeSource().getLocation().toURI().getPath();
            File jarDir = new File(path).getParentFile();
            if (jarDir != null) return jarDir.getAbsolutePath();
        } catch (Exception e) { }
        return System.getProperty("user.dir");
    }

    private static boolean isWindows() {
        return System.getProperty("os.name", "").toLowerCase().contains("win");
    }

    private static String detectLlamaBinary(String baseDir, Map<String, String> config) {
        // If llamacpp_dir is configured, look there first
        String llamacppDir = config.get("llamacpp_dir");
        if (llamacppDir != null && !llamacppDir.trim().isEmpty()) {
            llamacppDir = llamacppDir.trim();
            String exeName = isWindows() ? "llama-server.exe" : "llama-server";
            // If llamacpp_dir points directly to the executable
            File direct = new File(llamacppDir);
            if (direct.exists() && !direct.isDirectory()) {
                System.out.println("[信息] 使用配置的 llama-server: " + direct.getAbsolutePath());
                return direct.getAbsolutePath();
            }
            // If llamacpp_dir is a directory, look for the executable inside it
            File exe = new File(llamacppDir, exeName);
            if (exe.exists()) {
                System.out.println("[信息] 使用配置的 llama-server: " + exe.getAbsolutePath());
                return exe.getAbsolutePath();
            }
            System.err.println("[警告] 配置的 llamacpp_dir 中未找到 llama-server: " + llamacppDir);
        }

        // On non-Windows, prefer llama-server from PATH (brew install llama.cpp)
        if (!isWindows() && isCommandAvailable("llama-server")) {
            return "llama-server";
        }

        // Check bundled llama-b*-bin-win-* directory (Windows distribution)
        File baseDirFile = new File(baseDir);
        File[] subdirs = baseDirFile.listFiles(File::isDirectory);
        if (subdirs != null) {
            for (File d : subdirs) {
                if (d.getName().startsWith("llama-") && d.getName().contains("-bin-")) {
                    String exeName = isWindows() ? "llama-server.exe" : "llama-server";
                    File exe = new File(d, exeName);
                    if (exe.exists()) return exe.getAbsolutePath();
                }
            }
        }

        // Check other known locations
        String[] candidates = isWindows()
            ? new String[]{
                baseDir + File.separator + "lib" + File.separator + "llama-server.exe",
                baseDir + File.separator + "llama-server.exe",
              }
            : new String[]{
                baseDir + File.separator + "lib" + File.separator + "llama-server",
                baseDir + File.separator + "llama-server",
              };

        for (String c : candidates) {
            try {
                if (new File(c).exists()) return c;
            } catch (Exception e) { }
        }

        // Final fallback: try PATH on any platform
        if (isCommandAvailable("llama-server")) return "llama-server";
        return null;
    }

    private static boolean isCommandAvailable(String cmd) {
        try {
            Process p = new ProcessBuilder(cmd, "--version").redirectErrorStream(true).start();
            p.waitFor(5, TimeUnit.SECONDS);
            return true;
        } catch (Exception e) {
            return false;
        }
    }

    private static Map<String, String> loadConfig(String baseDir) {
        Map<String, String> cfg = new HashMap<>();
        File cfgFile = new File(baseDir, "config.yaml");
        if (!cfgFile.exists()) cfgFile = new File("config.yaml");
        if (!cfgFile.exists()) return cfg;

        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(new FileInputStream(cfgFile), StandardCharsets.UTF_8))) {
            String line;
            while ((line = reader.readLine()) != null) {
                line = line.trim();
                if (line.isEmpty() || line.startsWith("#")) continue;
                int colon = line.indexOf(':');
                if (colon < 0) continue;
                String key = line.substring(0, colon).trim();
                String value = line.substring(colon + 1).trim();
                if (!key.isEmpty() && !value.isEmpty()) {
                    cfg.put(key, value);
                }
            }
            System.out.println("[信息] 已加载配置: " + cfgFile.getAbsolutePath());
        } catch (IOException e) {
            System.err.println("[警告] 读取配置文件失败: " + e.getMessage());
        }
        return cfg;
    }

    private String cfg(String key, String defaultValue) {
        return config.containsKey(key) ? config.get(key) : defaultValue;
    }

    private static String detectModel(String baseDir, String preferredName) {
        File modelsDir = new File(baseDir, "models");
        if (!modelsDir.exists()) modelsDir = new File("models");
        if (!modelsDir.exists()) return null;

        // If config specifies a model, try it first
        if (preferredName != null && !preferredName.isEmpty()) {
            File preferred = new File(modelsDir, preferredName);
            if (preferred.exists()) return preferred.getAbsolutePath();
            System.err.println("[警告] 配置的模型 '" + preferredName + "' 不存在，自动检测");
        }

        File[] ggufFiles = modelsDir.listFiles((dir, name) ->
                name.toLowerCase().endsWith(".gguf"));
        if (ggufFiles == null || ggufFiles.length == 0) return null;

        Arrays.sort(ggufFiles, (a, b) -> Long.compare(b.length(), a.length()));
        return ggufFiles[0].getAbsolutePath();
    }

    private void startLlamaServer() throws Exception {
        System.out.println("[信息] 正在启动 llama-server...");

        List<String> command = new ArrayList<>();
        command.add(llamaBinary);
        command.add("-m");
        command.add(modelPath);
        command.add("--host");
        command.add(LLAMA_HOST);
        command.add("--port");
        command.add(String.valueOf(LLAMA_PORT));
        command.add("-c");
        command.add(cfg("context_size", "4096"));
        command.add("--top-k");
        command.add(cfg("top_k", "20"));
        command.add("--repeat-penalty");
        command.add(cfg("repeat_penalty", "1.05"));

        ProcessBuilder pb = new ProcessBuilder(command);
        // Set working directory to llama binary's dir so DLLs are found (Windows)
        File llamaDir = new File(llamaBinary).getParentFile();
        if (llamaDir != null) pb.directory(llamaDir);
        pb.redirectErrorStream(true);
        llamaProcess = pb.start();

        // Drain output in background thread
        new Thread(() -> {
            try (BufferedReader reader = new BufferedReader(
                    new InputStreamReader(llamaProcess.getInputStream()))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    System.out.println("[llama] " + line);
                }
            } catch (IOException e) { }
        }, "llama-output-drain").start();

        // Wait for llama-server to be ready
        System.out.print("[信息] 等待 llama-server 就绪");
        for (int i = 0; i < 60; i++) {
            Thread.sleep(1000);
            if (!llamaProcess.isAlive()) {
                throw new RuntimeException("llama-server 进程意外退出");
            }
            if (isLlamaReady()) {
                System.out.println(" 就绪!");
                return;
            }
            System.out.print(".");
        }
        throw new RuntimeException("llama-server 启动超时（60秒）");
    }

    private boolean isLlamaReady() {
        try {
            URL url = new URL("http://" + LLAMA_HOST + ":" + LLAMA_PORT + "/health");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod("GET");
            conn.setConnectTimeout(1000);
            conn.setReadTimeout(1000);
            return conn.getResponseCode() == 200;
        } catch (Exception e) {
            return false;
        }
    }

    private void startHttpServer() throws Exception {
        httpServer = HttpServer.create(new InetSocketAddress(SERVER_PORT), 0);
        httpServer.setExecutor(Executors.newFixedThreadPool(4));

        httpServer.createContext("/api/translate", new TranslateHandler());
        httpServer.createContext("/api/health", this::handleHealth);
        httpServer.createContext("/", new StaticFileHandler(baseDir));

        httpServer.start();
    }

    private void handleHealth(HttpExchange exchange) throws IOException {
        boolean ready = isLlamaReady();
        String response = "{\"ready\":" + ready + "}";
        sendJson(exchange, 200, response);
    }

    private void shutdown() {
        System.out.println("\n[信息] 正在关闭...");
        if (httpServer != null) httpServer.stop(2);
        if (llamaProcess != null && llamaProcess.isAlive()) {
            llamaProcess.destroyForcibly();
        }
        System.out.println("[信息] 已关闭");
    }

    // --- Translation Handler ---

    class TranslateHandler implements HttpHandler {
        @Override
        public void handle(HttpExchange exchange) throws IOException {
            if ("OPTIONS".equals(exchange.getRequestMethod())) {
                setCORS(exchange);
                exchange.sendResponseHeaders(204, -1);
                return;
            }

            if (!"POST".equals(exchange.getRequestMethod())) {
                sendError(exchange, 405, "仅支持 POST 请求");
                return;
            }

            try {
                String body = readBody(exchange);
                Map<String, String> params = parseJson(body);

                String text = params.get("text");
                String direction = params.get("direction");

                if (text == null || text.trim().isEmpty()) {
                    sendError(exchange, 400, "请输入要翻译的文本");
                    return;
                }
                if (direction == null) direction = "en2zh";

                String userPrompt = buildUserPrompt(text.trim(), direction);
                String llamaResponse = callLlamaApi(null, userPrompt);
                String result = extractContent(llamaResponse);

                String json;
                if ("en2both".equals(direction)) {
                    String userPromptEn = "Translate the following text into English, output only the translation:\n\n" + text.trim();
                    String llamaResponseEn = callLlamaApi(null, userPromptEn);
                    String resultEn = extractContent(llamaResponseEn);
                    json = "{\"result_zh\":" + escapeJson(result) + ",\"result_en\":" + escapeJson(resultEn) + "}";
                } else {
                    json = "{\"result\":" + escapeJson(result) + "}";
                }
                sendJson(exchange, 200, json);

            } catch (Exception e) {
                e.printStackTrace();
                sendError(exchange, 500, "翻译失败: " + e.getMessage());
            }
        }
    }

    private String buildUserPrompt(String text, String direction) {
        switch (direction) {
            case "zh2en":
                return "Translate the following segment into English, without additional explanation.\n\n" + text;
            case "en2both":
                return "Translate the following segment into Chinese, without additional explanation.\n\n" + text;
            case "de2en":
                return "Translate the following segment into English, without additional explanation.\n\n" + text;
            case "en2zh":
            default:
                return "Translate the following segment into Chinese, without additional explanation.\n\n" + text;
        }
    }

    private String callLlamaApi(String userPrompt) throws Exception {
        return callLlamaApi(null, userPrompt);
    }

    private String callLlamaApi(String systemPrompt, String userText) throws Exception {
        URL url = new URL("http://" + LLAMA_HOST + ":" + LLAMA_PORT + "/v1/chat/completions");
        HttpURLConnection conn = (HttpURLConnection) url.openConnection();
        conn.setRequestMethod("POST");
        conn.setRequestProperty("Content-Type", "application/json");
        conn.setDoOutput(true);
        conn.setConnectTimeout(5000);
        conn.setReadTimeout(120000);

        StringBuilder messages = new StringBuilder("[");
        if (systemPrompt != null) {
            messages.append("{\"role\":\"system\",\"content\":")
                    .append(escapeJson(systemPrompt)).append("},");
        }
        messages.append("{\"role\":\"user\",\"content\":")
                .append(escapeJson(userText)).append("}]");

        String requestBody = "{"
                + "\"messages\":" + messages.toString()
                + ",\"temperature\":" + cfg("temperature", "0.7")
                + ",\"top_p\":" + cfg("top_p", "0.6")
                + ",\"max_tokens\":" + cfg("max_tokens", "2048")
                + ",\"stream\":false"
                + "}";

        try (OutputStream os = conn.getOutputStream()) {
            os.write(requestBody.getBytes(StandardCharsets.UTF_8));
        }

        if (conn.getResponseCode() != 200) {
            String errBody = readStream(conn.getErrorStream());
            throw new RuntimeException("llama-server 返回 " + conn.getResponseCode() + ": " + errBody);
        }

        return readStream(conn.getInputStream());
    }

    private String extractContent(String jsonResponse) {
        // Extract from /v1/chat/completions response: choices[0].message.content
        int idx = jsonResponse.indexOf("\"content\"");
        if (idx < 0) {
            // Fallback: try "content" from /completion response
            idx = jsonResponse.indexOf("\"content\"");
            if (idx < 0) return jsonResponse;
        }

        // Find the content value after "content":
        // Skip past "content" and ":"
        int colonIdx = jsonResponse.indexOf(':', idx);
        if (colonIdx < 0) return jsonResponse;

        int start = colonIdx + 1;
        // Skip whitespace
        while (start < jsonResponse.length() && jsonResponse.charAt(start) == ' ') start++;

        if (start >= jsonResponse.length()) return jsonResponse;

        char first = jsonResponse.charAt(start);
        if (first == '"') {
            // String value
            start++;
            StringBuilder sb = new StringBuilder();
            for (int i = start; i < jsonResponse.length(); i++) {
                char c = jsonResponse.charAt(i);
                if (c == '\\' && i + 1 < jsonResponse.length()) {
                    char next = jsonResponse.charAt(++i);
                    switch (next) {
                        case 'n': sb.append('\n'); break;
                        case 't': sb.append('\t'); break;
                        case '"': sb.append('"'); break;
                        case '\\': sb.append('\\'); break;
                        case 'u': {
                            if (i + 4 < jsonResponse.length()) {
                                String hex = jsonResponse.substring(i + 1, i + 5);
                                try {
                                    sb.append((char) Integer.parseInt(hex, 16));
                                    i += 4;
                                } catch (NumberFormatException e) {
                                    sb.append("\\u").append(hex);
                                    i += 4;
                                }
                            }
                            break;
                        }
                        default: sb.append(next);
                    }
                } else if (c == '"') {
                    break;
                } else {
                    sb.append(c);
                }
            }
            return sb.toString().trim();
        }
        return jsonResponse;
    }

    // --- Static File Handler ---

    class StaticFileHandler implements HttpHandler {
        private final String webDir;

        StaticFileHandler(String baseDir) {
            this.webDir = baseDir + File.separator + "web";
        }

        @Override
        public void handle(HttpExchange exchange) throws IOException {
            String path = exchange.getRequestURI().getPath();
            if ("/".equals(path)) path = "/index.html";

            File file = new File(webDir, path);
            if (!file.exists() || file.isDirectory()) {
                file = new File(webDir, "index.html");
            }

            if (!file.exists()) {
                String msg = "404 Not Found";
                exchange.sendResponseHeaders(404, msg.getBytes(StandardCharsets.UTF_8).length);
                try (OutputStream os = exchange.getResponseBody()) {
                    os.write(msg.getBytes(StandardCharsets.UTF_8));
                }
                return;
            }

            String mime = getMimeType(file.getName());
            exchange.getResponseHeaders().set("Content-Type", mime + "; charset=utf-8");
            byte[] data = Files.readAllBytes(file.toPath());
            exchange.sendResponseHeaders(200, data.length);
            try (OutputStream os = exchange.getResponseBody()) {
                os.write(data);
            }
        }

        private String getMimeType(String name) {
            if (name.endsWith(".html")) return "text/html";
            if (name.endsWith(".css")) return "text/css";
            if (name.endsWith(".js")) return "application/javascript";
            if (name.endsWith(".json")) return "application/json";
            if (name.endsWith(".png")) return "image/png";
            if (name.endsWith(".jpg") || name.endsWith(".jpeg")) return "image/jpeg";
            if (name.endsWith(".svg")) return "image/svg+xml";
            if (name.endsWith(".ico")) return "image/x-icon";
            return "application/octet-stream";
        }
    }

    // --- Utility Methods ---

    private String readBody(HttpExchange exchange) throws IOException {
        return readStream(exchange.getRequestBody());
    }

    private String readStream(InputStream is) throws IOException {
        ByteArrayOutputStream bos = new ByteArrayOutputStream();
        byte[] buf = new byte[4096];
        int n;
        while ((n = is.read(buf)) != -1) bos.write(buf, 0, n);
        return bos.toString("UTF-8");
    }

    private void setCORS(HttpExchange exchange) {
        exchange.getResponseHeaders().set("Access-Control-Allow-Origin", "*");
        exchange.getResponseHeaders().set("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
        exchange.getResponseHeaders().set("Access-Control-Allow-Headers", "Content-Type");
    }

    private void sendJson(HttpExchange exchange, int code, String json) throws IOException {
        setCORS(exchange);
        exchange.getResponseHeaders().set("Content-Type", "application/json; charset=utf-8");
        byte[] data = json.getBytes(StandardCharsets.UTF_8);
        exchange.sendResponseHeaders(code, data.length);
        try (OutputStream os = exchange.getResponseBody()) {
            os.write(data);
        }
    }

    private void sendError(HttpExchange exchange, int code, String message) throws IOException {
        sendJson(exchange, code, "{\"error\":" + escapeJson(message) + "}");
    }

    private String escapeJson(String s) {
        if (s == null) return "null";
        StringBuilder sb = new StringBuilder("\"");
        for (char c : s.toCharArray()) {
            switch (c) {
                case '"': sb.append("\\\""); break;
                case '\\': sb.append("\\\\"); break;
                case '\n': sb.append("\\n"); break;
                case '\r': sb.append("\\r"); break;
                case '\t': sb.append("\\t"); break;
                default:
                    if (c < 0x20) {
                        sb.append(String.format("\\u%04x", (int) c));
                    } else {
                        sb.append(c);
                    }
            }
        }
        return sb.append("\"").toString();
    }

    @SuppressWarnings("unchecked")
    private Map<String, String> parseJson(String json) {
        Map<String, String> map = new HashMap<>();
        json = json.trim();
        if (json.startsWith("{")) json = json.substring(1);
        if (json.endsWith("}")) json = json.substring(0, json.length() - 1);

        StringBuilder key = new StringBuilder();
        StringBuilder val = new StringBuilder();
        boolean inKey = false, inVal = false, inStr = false;
        char strChar = 0;

        for (int i = 0; i < json.length(); i++) {
            char c = json.charAt(i);
            if (inStr) {
                if (c == '\\' && i + 1 < json.length()) {
                    char next = json.charAt(++i);
                    switch (next) {
                        case 'n': val.append('\n'); break;
                        case 't': val.append('\t'); break;
                        case '"': val.append('"'); break;
                        case '\\': val.append('\\'); break;
                        default: val.append(next);
                    }
                } else if (c == strChar) {
                    inStr = false;
                    if (inKey) { map.put(key.toString().trim(), ""); inKey = false; }
                    if (inVal) { map.put(key.toString().trim(), val.toString().trim()); inVal = false; }
                } else {
                    (inKey ? key : val).append(c);
                }
            } else if (c == '"') {
                inStr = true;
                strChar = '"';
                if (!inVal) inKey = true;
            } else if (c == ':') {
                inKey = false;
                inVal = true;
                key = new StringBuilder(key.toString().trim());
                val = new StringBuilder();
            } else if (c == ',') {
                if (inVal) {
                    map.put(key.toString().trim(), val.toString().trim());
                }
                key = new StringBuilder();
                val = new StringBuilder();
                inKey = false;
                inVal = false;
            } else if (inVal && !inStr) {
                val.append(c);
            } else if (inKey && !inStr) {
                key.append(c);
            }
        }
        if (inVal) {
            map.put(key.toString().trim(), val.toString().trim());
        }
        return map;
    }
}
