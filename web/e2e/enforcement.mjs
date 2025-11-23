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

async function register(driver, username) {
  await driver.get(`${baseUrl}/register`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(until.urlIs(`${baseUrl}/`), 5000)
}

async function signIn(driver, username) {
  await driver.get(`${baseUrl}/sign-in`)
  await driver.wait(until.elementLocated(By.name('username')), 5000)
  await driver.findElement(By.name('username')).sendKeys(username)
  await driver.findElement(By.name('password')).sendKeys('correcthorse')
  await driver.findElement(By.css('button[type="submit"]')).click()
  await driver.wait(async () => {
    if ((await driver.getCurrentUrl()) === `${baseUrl}/`) return true
    return (await driver.findElements(By.css('[role="alert"]'))).length > 0
  }, 15000, 'sign-in should either succeed or report an error')
}

function button(text) {
  return By.xpath(`//button[normalize-space(.)='${text}']`)
}

async function mainText(driver) {
  return (await driver.findElement(By.css('main')).getText()).replace(/\s+/g, ' ')
}

async function run() {
  const mod = await buildDriver()
  const suffix = Date.now().toString(36)
  const modName = `e2e_enf_m_${suffix}`
  const loudName = `e2e_enf_l_${suffix}`
  const topicTitle = `Enforced subject ${suffix}`
  const loudComment = `A loud opinion ${suffix}`
  let loud
  try {
    await register(mod, modName)
    await promoteViaRoot(modName)
    await mod.get(`${baseUrl}/s/general`)
    await mod.wait(until.elementLocated(button('New topic')), 5000)
    await mod.findElement(button('New topic')).click()
    await mod.wait(until.elementLocated(By.name('title')), 10000)
    await mod.findElement(By.name('title')).sendKeys(topicTitle)
    await mod.findElement(By.name('body')).sendKeys('Body for the enforcement e2e spec.')
    await mod.findElement(button('Post')).click()
    await mod.wait(until.elementLocated(By.linkText(topicTitle)), 5000)
    await mod.findElement(By.linkText(topicTitle)).click()
    await mod.wait(until.elementLocated(By.name('comment-body')), 5000)
    const topicUrl = await mod.getCurrentUrl()

    loud = await buildDriver()
    await register(loud, loudName)
    await loud.get(topicUrl)
    await loud.wait(until.elementLocated(By.name('comment-body')), 5000)
    await loud.findElement(By.name('comment-body')).sendKeys(loudComment)
    await loud.findElement(button('Post comment')).click()
    await loud.wait(until.elementLocated(By.xpath(`//p[contains(., '${loudComment}')]`)), 5000)

    await loud.get(`${baseUrl}/settings`)
    await loud.wait(until.elementLocated(By.css('main')), 5000)
    await loud.sleep(600)
    let text = await mainText(loud)
    assert(text.includes('No warnings.'), `a new user should have no warnings, saw: ${text}`)

    await mod.get(`${baseUrl}/u/${loudName}`)
    await mod.wait(until.elementLocated(button('Warn user')), 5000)
    await mod.findElement(button('Warn user')).click()
    const warnReason = await mod.wait(until.elementLocated(By.name('warn-reason')), 5000)
    await warnReason.sendKeys('please be civil')
    await mod.findElement(button('Send warning')).click()
    await mod.wait(until.elementLocated(By.css('[role="status"], [role="alert"]')), 10000)
    text = await mainText(mod)
    assert(text.includes('Warning sent.'), `warning should be confirmed, saw: ${text}`)

    await loud.get(`${baseUrl}/settings`)
    await loud.wait(until.elementLocated(button('Acknowledge warnings')), 5000)
    text = await mainText(loud)
    assert(text.includes('please be civil'), `the warned user should see the reason, saw: ${text}`)
    await loud.findElement(button('Acknowledge warnings')).click()
    await loud.sleep(1000)

    await mod.get(topicUrl)
    await mod.wait(until.elementLocated(By.xpath(`//p[contains(., '${loudComment}')]`)), 5000)
    await mod.get(`${baseUrl}/u/${loudName}`)
    await mod.wait(until.elementLocated(button('Ignore user')), 5000)
    await mod.findElement(button('Ignore user')).click()
    await mod.wait(until.elementLocated(button('Stop ignoring')), 5000)

    await mod.get(topicUrl)
    await mod.wait(until.elementLocated(By.name('comment-body')), 5000)
    await mod.sleep(800)
    text = await mainText(mod)
    assert(
      text.includes('Hidden — you ignore this author.'),
      `an ignored author's comment should be hidden, saw: ${text}`,
    )
    assert(!text.includes(loudComment), 'the ignored body must not be shown')

    const loudView = await (async () => {
      await loud.get(topicUrl)
      await loud.wait(until.elementLocated(By.name('comment-body')), 5000)
      await loud.sleep(600)
      return mainText(loud)
    })()
    assert(
      loudView.includes(loudComment),
      'ignoring is per-viewer; the author still sees their own comment',
    )

    await mod.get(`${baseUrl}/u/${loudName}`)
    await mod.wait(until.elementLocated(button('Stop ignoring')), 5000)
    await mod.findElement(button('Stop ignoring')).click()
    await mod.wait(until.elementLocated(button('Ignore user')), 5000)

    await mod.get(`${baseUrl}/u/${loudName}`)
    await mod.wait(until.elementLocated(button('Ban user')), 5000)
    await mod.findElement(button('Ban user')).click()
    const banReason = await mod.wait(until.elementLocated(By.name('ban-reason')), 5000)
    await banReason.sendKeys('repeated spam')
    await mod.findElement(button('Confirm ban')).click()
    await mod.wait(until.elementLocated(By.css('[role="status"], [role="alert"]')), 10000)
    text = await mainText(mod)
    assert(text.includes('User banned.'), `the ban should be confirmed, saw: ${text}`)

    await loud.get(`${baseUrl}/`)
    await loud.wait(
      async () => (await loud.findElement(By.css('nav')).getText()).includes('Sign in'),
      15000,
      'a banned session should stop working',
    )
    let nav = await loud.findElement(By.css('nav')).getText()
    assert(nav.includes('Sign in'), `a banned user's session should stop working, nav: ${nav}`)

    await signIn(loud, loudName)
    text = await mainText(loud)
    assert(
      text.toLowerCase().includes('suspended'),
      `a banned user should be told why sign-in failed, saw: ${text}`,
    )

    await mod.get(`${baseUrl}/u/${loudName}`)
    await mod.wait(until.elementLocated(button('Lift ban')), 5000)
    await mod.findElement(button('Lift ban')).click()
    await mod.sleep(1500)

    await signIn(loud, loudName)
    nav = await loud.findElement(By.css('nav')).getText()
    assert(
      nav.includes(loudName),
      `lifting the ban should let them sign in again, nav: ${nav}`,
    )

    console.log('e2e: enforcement flow passed')
  } finally {
    await mod.quit()
    if (loud) await loud.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
