import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { promoteViaRoot } from './support.mjs'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

async function mainText(driver) {
  let last
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
    } catch (err) {
      last = err
      await new Promise((resolve) => setTimeout(resolve, 250))
    }
  }
  throw last
}

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 15000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 15000)
}

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 15000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 15000)
}

async function countOf(driver, what) {
  const found = await driver.findElements(By.css(`.count[data-of="${what}"]`))
  if (found.length === 0) return null
  const text = await found[0].getText()
  return text === '—' ? null : Number(text)
}

async function run() {
  const mod = await buildDriver()
  const plain = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_op_m_${suffix}`
  const plainName = `e2e_op_p_${suffix}`
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await signIn(mod, modName)
    await register(plain, plainName)

    await plain.get(`${baseUrl}/operator`)
    await plain.wait(
      async () => (await mainText(plain)).includes('Only a moderator can run the board.'),
      15000,
      'a plain reader must be turned away',
    )
    assert(
      (await plain.findElements(By.linkText('Operator'))).length === 0,
      'a plain reader is not offered the door either',
    )

    await mod.get(`${baseUrl}/`)
    await mod.wait(
      until.elementLocated(By.linkText('Operator')),
      15000,
      'a moderator should be offered one door to the controls',
    )
    await mod.findElement(By.linkText('Operator')).click()
    await mod.wait(
      async () => (await mainText(mod)).includes('Open reports'),
      15000,
      'the surface should say what the board looks like',
    )

    await mod.wait(
      async () => (await countOf(mod, 'sections')) !== null,
      15000,
      'the counts should arrive',
    )
    const sections = await countOf(mod, 'sections')
    assert(sections !== null && sections > 0, `a board has at least one section, saw: ${sections}`)
    const reports = await countOf(mod, 'reports')
    assert(reports !== null, 'the open report count should be answered')

    const text = await mainText(mod)
    for (const label of ['Open reports', 'Blocked addresses', 'Sections']) {
      assert(text.includes(label), `the surface should list ${label}, saw: ${text}`)
    }
    assert(text.includes('Run maintenance'), `maintenance belongs here, saw: ${text}`)

    for (const [label, path] of [
      ['Open reports', '/reports'],
      ['Blocked addresses', '/addresses'],
      ['Sections', '/section-settings'],
    ]) {
      await mod.get(`${baseUrl}/operator`)
      await mod.wait(until.elementLocated(By.linkText(label)), 15000, `the ${label} link`)
      await mod.findElement(By.linkText(label)).click()
      await mod.wait(
        async () => (await mod.getCurrentUrl()).includes(path),
        15000,
        `${label} should lead to ${path}`,
      )
    }

    await mod.get(`${baseUrl}/settings`)
    await mod.wait(
      async () => (await mainText(mod)).includes('Display'),
      15000,
      'personal settings should still be there',
    )
    assert(
      (await mod.findElements(By.xpath("//button[text()='Run maintenance']"))).length === 0,
      'running the board is not a personal setting',
    )

    console.log('e2e: operator surface flow passed')
  } finally {
    await mod.quit()
    await plain.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
