(function() {
  var sourceText = document.getElementById('source-text');
  var resultText = document.getElementById('result-text');
  var translateBtn = document.getElementById('translate-btn');
  var clearBtn = document.getElementById('clear-btn');
  var copyBtn = document.getElementById('copy-btn');
  var statusEl = document.getElementById('status');
  var sourceCount = document.getElementById('source-count');
  var resultCount = document.getElementById('result-count');
  var timeInfo = document.getElementById('time-info');
  var sourceLang = document.getElementById('source-lang');
  var targetLang = document.getElementById('target-lang');
  var detectInfo = document.getElementById('detect-info');

  function detectLanguage(text) {
    if (!text || !text.trim()) return null;
    var chineseCount = 0;
    var total = 0;
    for (var i = 0; i < text.length; i++) {
      var code = text.charCodeAt(i);
      if (code >= 0x4e00 && code <= 0x9fff) {
        chineseCount++;
        total++;
      } else if ((code >= 0x41 && code <= 0x5a) || (code >= 0x61 && code <= 0x7a)) {
        total++;
      }
    }
    if (total === 0) return null;
    if (chineseCount / total > 0.3) return 'zh';
    return 'non-zh';
  }

  function updateDetectInfo() {
    var text = sourceText.value.trim();
    var lang = detectLanguage(text);
    if (!lang) {
      sourceLang.textContent = '输入文本';
      targetLang.textContent = '翻译结果';
      detectInfo.textContent = '';
      return;
    }
    if (lang === 'zh') {
      sourceLang.textContent = '中文';
      targetLang.textContent = 'English';
      detectInfo.textContent = '已识别: 中文 → 英语';
    } else {
      sourceLang.textContent = '外文';
      targetLang.textContent = '中文 + English';
      detectInfo.textContent = '已识别: 外文 → 中文 & 英语';
    }
  }

  function updateCharCount(textarea, countEl) {
    var len = textarea.value.length;
    countEl.textContent = len + ' 字符';
  }

  function setStatus(text, cls) {
    statusEl.textContent = text;
    statusEl.className = cls;
  }

  function translate() {
    var text = sourceText.value.trim();
    if (!text) {
      resultText.value = '';
      resultText.placeholder = '请先输入要翻译的文本';
      return;
    }

    var lang = detectLanguage(text);
    var direction;
    if (lang === 'zh') {
      direction = 'zh2en';
    } else {
      direction = 'en2both';
    }

    translateBtn.disabled = true;
    translateBtn.textContent = '翻译中...';
    resultText.value = '';
    timeInfo.textContent = '';

    var startTime = Date.now();

    fetch('/api/translate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ text: text, direction: direction })
    })
    .then(function(resp) { return resp.json(); })
    .then(function(data) {
      var elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      if (data.error) {
        resultText.value = '';
        resultText.placeholder = '错误: ' + data.error;
        timeInfo.textContent = '';
      } else if (data.result_zh && data.result_en) {
        resultText.value = '【中文翻译】\n' + data.result_zh + '\n\n【English Translation】\n' + data.result_en;
        timeInfo.textContent = '耗时 ' + elapsed + ' 秒';
      } else {
        resultText.value = data.result || '';
        timeInfo.textContent = '耗时 ' + elapsed + ' 秒';
      }
      updateCharCount(resultText, resultCount);
    })
    .catch(function(err) {
      resultText.value = '';
      resultText.placeholder = '请求失败: ' + err.message;
      setStatus('连接失败', 'status-error');
    })
    .finally(function() {
      translateBtn.disabled = false;
      translateBtn.textContent = '翻译';
    });
  }

  function copyResult() {
    var text = resultText.value;
    if (!text) return;
    if (navigator.clipboard) {
      navigator.clipboard.writeText(text).then(function() {
        copyBtn.textContent = '已复制!';
        setTimeout(function() { copyBtn.textContent = '复制'; }, 1500);
      });
    } else {
      resultText.select();
      document.execCommand('copy');
      copyBtn.textContent = '已复制!';
      setTimeout(function() { copyBtn.textContent = '复制'; }, 1500);
    }
  }

  function checkHealth() {
    fetch('/api/health')
      .then(function(resp) { return resp.json(); })
      .then(function(data) {
        if (data.ready) {
          setStatus('已连接 - 就绪', 'status-connected');
        } else {
          setStatus('翻译引擎未就绪', 'status-error');
        }
      })
      .catch(function() {
        setStatus('未连接', 'status-disconnected');
      });
  }

  translateBtn.addEventListener('click', translate);
  clearBtn.addEventListener('click', function() {
    sourceText.value = '';
    updateCharCount(sourceText, sourceCount);
    updateDetectInfo();
  });
  copyBtn.addEventListener('click', copyResult);
  sourceText.addEventListener('input', function() {
    updateCharCount(sourceText, sourceCount);
    updateDetectInfo();
  });

  sourceText.addEventListener('keydown', function(e) {
    // IME composition: Enter confirms a candidate — must not trigger translate.
    if (e.isComposing || e.keyCode === 229) return;
    if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      translate();
    }
  });

  updateDetectInfo();
  checkHealth();
  setInterval(checkHealth, 30000);
})();
