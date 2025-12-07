import { Builder, By, logging } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { placesFor } from './places.mjs'

const chromedriverPath = process.env.CHROMEDRIVER_PATH

const WIRE_WORDS = [
  'missing session',
  'unauthorized',
  'internal server error',
  'null',
  'undefined',
  'exception',
  'stack trace',
]

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu', '--window-size=1280,900')
  const prefs = new logging.Preferences()
  prefs.setLevel(logging.Type.BROWSER, logging.Level.ALL)
  options.setLoggingPrefs(prefs)
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) builder.setChromeService(new chrome.ServiceBuilder(chromedriverPath))
  return builder.build()
}

async function textOf(driver, selector) {
  const found = await driver.findElements(By.css(selector))
  if (found.length === 0) return null
  return (await found[0].getText()).replace(/\s+/g, ' ').trim()
}

async function ask(driver, base, place) {
  await driver.get(`${base}${place.at}`)
  await new Promise((r) => setTimeout(r, 1200))

  const title = (await driver.getTitle()) ?? ''
  const headings = await driver.findElements(By.css('h1'))
  const headingTexts = []
  for (const h of headings) headingTexts.push((await h.getText()).trim())
  const body = (await textOf(driver, 'body')) ?? ''
  const anchors = await driver.findElements(By.css('a[href]'))

  let severe = 0
  let complaints = []
  try {
    const entries = await driver.manage().logs().get(logging.Type.BROWSER)
    const bad = entries.filter(
      (e) => e.level.name === 'SEVERE' && !e.message.includes('favicon'),
    )
    severe = bad.length
    complaints = bad.map((e) => e.message.replace(/\s+/g, ' ').slice(0, 120))
  } catch {
    severe = 0
  }

  const lower = body.toLowerCase()
  const leaked = WIRE_WORDS.filter((word) => lower.includes(word))

  return {
    what: place.what,
    titled: title.length > 0,
    title,
    headed: headingTexts.filter((t) => t.length > 0).length > 0,
    heading: headingTexts[0] ?? '',
    saysSomething: body.length > 40,
    wordCount: body.length,
    waysOut: anchors.length,
    leaked,
    severe,
    complaints,
  }
}

async function walk(which) {
  const { name, base, places } = placesFor(which)
  const driver = await buildDriver()
  const answers = []
  try {
    for (const place of places) {
      answers.push(await ask(driver, base, place))
    }
  } finally {
    await driver.quit()
  }
  return { name, answers }
}

function verdict(a) {
  const faults = []
  if (!a.titled) faults.push('the tab does not name it')
  if (!a.headed) faults.push('no heading says where you are')
  if (!a.saysSomething) faults.push(`almost nothing on the page (${a.wordCount} characters)`)
  if (a.waysOut === 0) faults.push('no way onward from here')
  if (a.leaked.length > 0) faults.push(`shows the wire's words: ${a.leaked.join(', ')}`)
  for (const complaint of a.complaints) faults.push(`the console says: ${complaint}`)
  return faults
}

const which = process.argv[2] ?? 'clone'
const result = await walk(which)

console.log(`\n== ${result.name} ==`)
for (const a of result.answers) {
  const faults = verdict(a)
  const mark = faults.length === 0 ? 'ok  ' : 'FAIL'
  console.log(`${mark} ${a.what}`)
  console.log(`       tab: ${a.title || '(none)'}`)
  console.log(`       heading: ${a.heading || '(none)'}`)
  for (const fault of faults) console.log(`       - ${fault}`)
}
console.log('COMPAREDONE')
