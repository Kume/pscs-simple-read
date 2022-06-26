const {execSync} = require('child_process');
const path = require('path');

const cards = [
  {name: 'カード1', id: [4,   3,  53, 122, 19, 111, 129]},
  {name: 'カード2', id: [4,  56,  71, 122, 19, 111, 129]},
  {name: 'カード3', id: [4, 123,  85, 122, 19, 111, 129]},
]

const blackCardAttribute = [59,143,128,1,128,79,12,160,0,0,3,6,3,0,3,0,0,0,0,104];
const felicaAttribute = [59,143,128,1,128,79,12,160,0,0,3,6,17,0,59,0,0,0,0,66];

function arrayEquals(a, b) {
  if (a.length != b.length) {
    return false;
  }
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) {
      return false;
    }
  }
  return true;
}

function exec() {
  console.log('読み取り開始');
  const resultBuffer = execSync('pcsc-simple-read', {cwd: path.resolve(__dirname, 'target/release')});
  const result = resultBuffer.toString('utf-8');
  const cardInfo = JSON.parse(result);
  console.log('読み出した値', JSON.parse(result));
  if (arrayEquals(cardInfo.attr, felicaAttribute)) {
    console.log('かざされたカードはFeliCaカードです。')
  }
  for (const card of cards) {
    if (arrayEquals(cardInfo.id, card.id)) {
      console.log(`かざされたカードは [${card.name}] です。`)
    }
  }
  setTimeout(exec, 2000);
}

exec();